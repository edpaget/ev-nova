//! Faded models drawn offscreen (`nova_view::DrawCommand::Model`'s alpha):
//! over a known backdrop, a cube turned so its +z and +y faces show draws
//! each face as its fragment faded to the model's alpha over the backdrop
//! (`camera::{fragment, faded}`), its own back faces and a part mounted
//! inside it never showing through it; an earlier model shows through a
//! faded one, and a later opaque one covers it; a model at alpha 0 draws
//! nothing, its parts included; and at alpha 1 it draws exactly as an
//! opaque model does.
//!
//! The alphas are chosen here: which alpha a cloak, a gate or a
//! `BaseTransp` gives is nova-view's (its flight `model_fade` module),
//! pinned there. Needs a GPU adapter; skips, passing, without one.

use std::f32::consts::FRAC_PI_6;

use nova_render::camera::{faded, fragment};
use nova_render::fonts::FontFaces;
use nova_render::{Gpu, Light, Mesh, RenderReport, Renderer, Upscaler, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, ModelKey, ModelPose, Mounting, Orientation, Point};

use super::common::fill;
use super::common::pixels::{LOGICAL, SIZE, assert_near, color_at};
use super::wgpu_offscreen::{Images, gpu_with};

const CUBE: ModelKey = ModelKey(1);
/// A cube half the size, white all over.
const PART: ModelKey = ModelKey(2);

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

/// The backdrop every scene is drawn over.
const BACKDROP: [u8; 3] = [40, 120, 60];

/// A point on the +z face, where the −y face lies behind it: drawn
/// before it, so a model that blended its back faces would show it.
const ON_PLUS_Z: (f32, f32) = (12.0, 7.5);
/// A point on the +y face, where the −z face lies behind it.
const ON_PLUS_Y: (f32, f32) = (12.0, 16.5);

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// The cube's turn: 30° about x, which tips +y toward the camera, so +z
/// and +y face it and −z and −y face away.
fn turn() -> Orientation {
    Orientation::about([1.0, 0.0, 0.0], FRAC_PI_6)
}

/// The cube at (`x`, 12), 10 logical units a side. Turned, at x 12 its +z
/// covers logical y 5.17 to 13.83 and its +y 13.83 to 18.83, and x 7 to
/// 17.
fn pose_at(x: f32) -> ModelPose {
    ModelPose {
        center: at(x, 12.0),
        orientation: turn(),
        scale: 10.0,
    }
}

fn cube_pose() -> ModelPose {
    pose_at(12.0)
}

/// The cube, in [`FACES`].
fn cube() -> Mesh {
    Mesh::cube(FACES.map(|[r, g, b]| Color::rgba(r, g, b, 255)))
}

/// A white cube half the cube's size.
fn part() -> Mesh {
    let white = Mesh::cube([Color::WHITE; 6]);
    let vertices = (white.vertices().iter())
        .map(|vertex| {
            let mut half = *vertex;
            half.position = vertex.position.map(|c| c * 0.5);
            half
        })
        .collect();
    Mesh::new(vertices, white.indices().to_vec()).expect("a cube")
}

/// The backdrop, over the whole logical screen.
fn backdrop(list: &mut DrawList) {
    let [r, g, b] = BACKDROP;
    let all = Bounds {
        min: at(0.0, 0.0),
        max: at(LOGICAL.w as f32, LOGICAL.h as f32),
    };
    fill(list, all, Color::rgba(r, g, b, 255));
}

/// The backdrop, then whatever `draw` adds.
fn scene(draw: impl FnOnce(&mut DrawList)) -> DrawList {
    let mut list = DrawList::new();
    backdrop(&mut list);
    draw(&mut list);
    list
}

/// `list` drawn with the cube and the part registered, read back; `None`
/// with no adapter.
fn drawn(list: &DrawList) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(Upscaler::Native);
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, cube());
    renderer.add_model(PART, part());
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// Face `face`'s fragment, turned, as the core lights it, faded to
/// `alpha` (`camera::faded`).
fn fragment_of(face: usize, alpha: f32) -> [f32; 4] {
    let normal = turn().rotate(NORMALS[face]).expect("a unit turn");
    let [r, g, b] = FACES[face].map(|c| f32::from(c) / 255.0);
    let color = [r, g, b, 1.0];
    faded(
        fragment(normal, color, [1.0; 4], false, Light::STANDARD),
        alpha,
    )
}

/// `fragment` blended straight over `beneath`, rounded to 8 bits.
fn over(beneath: [u8; 3], [r, g, b, a]: [f32; 4]) -> [u8; 3] {
    let [br, bg, bb] = beneath.map(|c| f32::from(c) / 255.0);
    [(r, br), (g, bg), (b, bb)].map(|(c, d)| ((c * a + d * (1.0 - a)) * 255.0).round() as u8)
}

#[test]
fn a_faded_cube_shows_each_face_faded_over_the_backdrop_alone() {
    for alpha in [0.25, 0.5, 0.6875] {
        let list = scene(|list| {
            list.faded_model(CUBE, cube_pose(), alpha);
        });
        let Some(pixels) = drawn(&list) else {
            return;
        };
        for (face, point, what) in [(PLUS_Z, ON_PLUS_Z, "+z"), (PLUS_Y, ON_PLUS_Y, "+y")] {
            let want = over(BACKDROP, fragment_of(face, alpha));
            assert_near(&pixels, point, want, &format!("{what} at {alpha}"));
        }
        assert_near(&pixels, (25.0, 12.0), BACKDROP, "beside it");
    }
}

/// A part at the cube's centre, wholly inside it.
fn inside() -> Mounting {
    Mounting {
        at: [0.0; 3],
        turn: 0.0,
    }
}

#[test]
fn a_part_inside_a_faded_hull_never_shows_through_it() {
    for alpha in [1.0, 0.5] {
        let list = scene(|list| {
            list.faded_model(CUBE, cube_pose(), alpha)
                .mounted(PART, inside());
        });
        let Some(pixels) = drawn(&list) else {
            return;
        };
        for (face, point) in [(PLUS_Z, ON_PLUS_Z), (PLUS_Y, ON_PLUS_Y)] {
            let want = over(BACKDROP, fragment_of(face, alpha));
            assert_near(&pixels, point, want, &format!("the hull at {alpha}"));
        }
    }
}

#[test]
fn an_earlier_model_shows_through_a_faded_one_and_a_later_one_covers_it() {
    let alpha = 0.5;
    // The first cube, opaque, left of the faded one, which overlaps its
    // right side; the faded cube's +z at x 12 is over the first's +z.
    let first = pose_at(6.0);
    let list = scene(|list| {
        list.model(CUBE, first)
            .faded_model(CUBE, cube_pose(), alpha);
    });
    let Some(pixels) = drawn(&list) else {
        return;
    };
    let beneath = over(BACKDROP, fragment_of(PLUS_Z, 1.0));
    assert_near(
        &pixels,
        (10.0, ON_PLUS_Z.1),
        over(beneath, fragment_of(PLUS_Z, alpha)),
        "the first cube through the faded one",
    );
    let covered = scene(|list| {
        list.faded_model(CUBE, cube_pose(), alpha)
            .model(CUBE, cube_pose());
    });
    let Some(pixels) = drawn(&covered) else {
        return;
    };
    for (face, point) in [(PLUS_Z, ON_PLUS_Z), (PLUS_Y, ON_PLUS_Y)] {
        assert_near(
            &pixels,
            point,
            over(BACKDROP, fragment_of(face, 1.0)),
            "the later cube over it",
        );
    }
}

/// A part standing out of the cube to its right: at logical x 24.
fn beside() -> Mounting {
    Mounting {
        at: [1.2, 0.0, 0.0],
        turn: 0.0,
    }
}

#[test]
fn a_model_at_alpha_0_draws_nothing_nor_its_parts() {
    let whole = scene(|list| {
        list.model(CUBE, cube_pose()).mounted(PART, beside());
    });
    let Some(shown) = drawn(&whole) else {
        return;
    };
    assert_ne!(
        color_at(&shown, 24.0, 12.0),
        BACKDROP,
        "the part shows whole"
    );
    let gone = scene(|list| {
        list.faded_model(CUBE, cube_pose(), 0.0)
            .mounted(PART, beside());
    });
    let Some(pixels) = drawn(&gone) else {
        return;
    };
    let Some(bare) = drawn(&scene(|_| {})) else {
        return;
    };
    assert_eq!(pixels, bare, "the backdrop alone");
}

#[test]
fn a_model_at_alpha_1_draws_exactly_as_an_opaque_model() {
    let faded = scene(|list| {
        list.faded_model(CUBE, cube_pose(), 1.0)
            .mounted(PART, beside());
    });
    let whole = scene(|list| {
        list.model(CUBE, cube_pose()).mounted(PART, beside());
    });
    let (Some(a), Some(b)) = (drawn(&faded), drawn(&whole)) else {
        return;
    };
    assert_eq!(a, b);
}
