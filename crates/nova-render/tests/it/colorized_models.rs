//! Colorized models drawn offscreen (`nova_view::DrawCommand::Model`'s
//! colorize): the `faded_models` scene's turned cube, drawn mixed toward a
//! colour, shows each face as the core colorizes its lit fragment
//! (`camera::{fragment, colorized, faded}`): mixed by its level, the flat
//! colour at level 32 whatever the light, untouched by a colorize that
//! doesn't mix, mixed and then faded over the backdrop at an alpha, a
//! part mixed as its hull is, and an unlit face mixed as a lit one.
//!
//! The colorizes are chosen here: which one a murk, an ion tint or a gate
//! gives is nova-view's (its flight view's `model_colorize` tests). Needs
//! a GPU adapter; skips, passing, without one.

use nova_render::camera::{colorized, faded, fragment};
use nova_render::{Light, Mesh};
use nova_view::{Color, Colorize, DrawCommand, DrawList, ModelPose};

use super::common::pixels::{assert_near, near};
use super::faded_models::{
    BACKDROP, CUBE, NORMALS, ON_PLUS_Y, ON_PLUS_Z, PART, PLUS_Y, PLUS_Z, beside, drawn_with,
    inside, over, pose_at, scene, turn,
};

/// The cube's faces' colours, in `Mesh::cube`'s order (+x, −x, +y, −y,
/// +z, −z): +y and +z chosen so each lit channel sits mid-way in a 5-bit
/// step ([`mid_step`]).
const FACES: [[u8; 3]; 6] = [
    [0, 255, 255],
    [255, 0, 255],
    [177, 232, 67],
    [255, 255, 0],
    [227, 47, 132],
    [40, 40, 255],
];

/// The colour of every face of the unlit cube: mid-way in its 5-bit steps
/// as it is.
const UNLIT: [u8; 3] = [203, 99, 35];

const WHITE: [u8; 3] = [255; 3];

/// Half-way toward a blue.
const HALF: Colorize = Colorize {
    level: 16,
    rgb5: [4, 20, 31],
};

/// All the way toward the same blue.
const FULL: Colorize = Colorize { level: 32, ..HALF };

/// The cube, in [`FACES`].
fn cube() -> Mesh {
    Mesh::cube(FACES.map(|[r, g, b]| Color::rgba(r, g, b, 255)))
}

/// A cube of [`UNLIT`] whose every vertex is unlit.
fn unlit_cube() -> Mesh {
    let [r, g, b] = UNLIT;
    let lit = Mesh::cube([Color::rgba(r, g, b, 255); 6]);
    let vertices = (lit.vertices().iter())
        .map(|vertex| nova_render::Vertex {
            unlit: true,
            ..*vertex
        })
        .collect();
    Mesh::new(vertices, lit.indices().to_vec()).expect("a cube")
}

/// `list` drawn with the cube in [`FACES`]; `None` with no adapter.
fn drawn(list: &DrawList) -> Option<Vec<u8>> {
    drawn_with(list, cube())
}

/// The cube at `pose` at `alpha`, colorized `colorize`.
fn mixed(pose: ModelPose, alpha: f32, colorize: Colorize) -> DrawCommand {
    DrawCommand::faded_model(CUBE, pose, alpha).with_colorize(Some(colorize))
}

/// A face of `color` facing `NORMALS[face]` on the turned cube, as the
/// core lights it, or leaves it when `unlit`.
fn fragment_of(face: usize, color: [u8; 3], unlit: bool) -> [f32; 4] {
    let normal = turn().rotate(NORMALS[face]).expect("a unit turn");
    let [r, g, b] = color.map(|c| f32::from(c) / 255.0);
    fragment(normal, [r, g, b, 1.0], [1.0; 4], unlit, Light::STANDARD)
}

/// `fragment`, having checked that each of its colour channels sits at
/// least one 8-bit step from where a 5-bit step turns over (8k − 0.5, k
/// from 1 to 31): the GPU interpolates its normal and may round a channel
/// apart from the core by a fraction, which must not move it to another
/// step.
#[track_caller]
fn mid_step(fragment: [f32; 4], what: &str) -> [f32; 4] {
    for channel in &fragment[..3] {
        let value = channel.clamp(0.0, 1.0) * 255.0;
        let turn = ((value + 0.5) / 8.0).round().clamp(1.0, 31.0) * 8.0 - 0.5;
        assert!(
            (value - turn).abs() >= 1.0,
            "{what}: {value} is on a 5-bit step's edge, {turn}"
        );
    }
    fragment
}

/// What a face showing `fragment` draws as, colorized `colorize` and
/// faded to `alpha` over the backdrop, as the core has it.
#[track_caller]
fn shown(fragment: [f32; 4], colorize: Colorize, alpha: f32, what: &str) -> [u8; 3] {
    over(
        BACKDROP,
        faded(colorized(mid_step(fragment, what), colorize), alpha),
    )
}

/// The cube's +z and +y faces: their fragments lit, where each shows on a
/// cube at x 12, and their names.
fn faces() -> [([f32; 4], (f32, f32), &'static str); 2] {
    [
        (fragment_of(PLUS_Z, FACES[PLUS_Z], false), ON_PLUS_Z, "+z"),
        (fragment_of(PLUS_Y, FACES[PLUS_Y], false), ON_PLUS_Y, "+y"),
    ]
}

#[test]
fn a_colorized_cube_shows_each_face_mixed_toward_the_colour_by_its_level() {
    for level in [16, 6, 27] {
        let colorize = Colorize { level, ..HALF };
        let list = scene(|list| {
            list.push(mixed(pose_at(12.0), 1.0, colorize));
        });
        let Some(pixels) = drawn(&list) else {
            return;
        };
        for (fragment, point, what) in faces() {
            let what = format!("{what} at level {level}");
            let want = shown(fragment, colorize, 1.0, &what);
            assert_near(&pixels, point, want, &what);
            // Neither its own colour nor the colorize's.
            assert!(!near(want, over(BACKDROP, fragment)), "{what}: mixed");
            assert!(
                !near(want, shown(fragment, FULL, 1.0, &what)),
                "{what}: not the colour alone"
            );
        }
        assert_near(&pixels, (25.0, 12.0), BACKDROP, "beside it");
    }
}

#[test]
fn at_level_32_every_face_is_the_colour() {
    let list = scene(|list| {
        list.push(mixed(pose_at(12.0), 1.0, FULL));
    });
    let Some(pixels) = drawn(&list) else {
        return;
    };
    let [r, g, b] = FULL.rgb5;
    let colour = Color::from_rgb555([r, g, b]);
    let colour = [colour.r, colour.g, colour.b];
    let [(front, ..), (top, ..)] = faces();
    assert!(
        !near(over(BACKDROP, front), over(BACKDROP, top)),
        "the faces are lit apart"
    );
    for (fragment, point, what) in faces() {
        assert_eq!(shown(fragment, FULL, 1.0, what), colour, "{what}");
        assert_near(&pixels, point, colour, what);
    }
}

#[test]
fn a_colorize_that_does_not_mix_draws_exactly_as_none() {
    let plain = scene(|list| {
        list.model(CUBE, pose_at(12.0)).mounted(PART, beside());
    });
    let Some(plain) = drawn(&plain) else {
        return;
    };
    for level in [0, 33] {
        // Built by hand: `with_colorize` would drop it.
        let list = scene(|list| {
            list.push(DrawCommand::Model {
                model: CUBE,
                pose: pose_at(12.0),
                alpha: 1.0,
                colorize: Some(Colorize { level, ..HALF }),
            })
            .mounted(PART, beside());
        });
        let Some(pixels) = drawn(&list) else {
            return;
        };
        assert_eq!(pixels, plain, "level {level}");
    }
}

#[test]
fn a_faded_colorized_cube_mixes_then_fades_over_the_backdrop() {
    let alpha = 0.5;
    let list = scene(|list| {
        list.push(mixed(pose_at(12.0), alpha, HALF))
            .mounted(PART, inside());
    });
    let Some(pixels) = drawn(&list) else {
        return;
    };
    for (fragment, point, what) in faces() {
        // The hull alone: the part inside it never shows through.
        let want = shown(fragment, HALF, alpha, what);
        assert_near(&pixels, point, want, what);
        assert!(
            !near(want, over(BACKDROP, faded(fragment, alpha))),
            "{what}: mixed"
        );
        assert!(
            !near(want, shown(fragment, HALF, 1.0, what)),
            "{what}: faded"
        );
    }
}

#[test]
fn a_part_takes_its_hulls_colorize() {
    // The colorized hull at x 6, its part standing out at x 18, and a
    // plain hull after them at x 26.
    let list = scene(|list| {
        list.push(mixed(pose_at(6.0), 1.0, HALF))
            .mounted(PART, beside())
            .model(CUBE, pose_at(26.0));
    });
    let Some(pixels) = drawn(&list) else {
        return;
    };
    let (z, y) = (ON_PLUS_Z.1, ON_PLUS_Y.1);
    for (fragment, _, what) in faces() {
        let at = if what == "+z" { z } else { y };
        let mixed = shown(fragment, HALF, 1.0, what);
        assert_near(&pixels, (6.0, at), mixed, &format!("the hull's {what}"));
        let plain = over(BACKDROP, fragment);
        assert_near(&pixels, (26.0, at), plain, &format!("the next's {what}"));
    }
    // The part, half the size: its +z about (18, 10.5).
    let white = fragment_of(PLUS_Z, WHITE, false);
    let want = shown(white, HALF, 1.0, "the part's +z");
    assert_near(&pixels, (18.0, 10.5), want, "the part's +z");
    assert!(!near(want, over(BACKDROP, white)), "the part is mixed");
}

#[test]
fn an_unlit_face_is_colorized_too() {
    let list = scene(|list| {
        list.push(mixed(pose_at(12.0), 1.0, HALF));
    });
    let Some(pixels) = drawn_with(&list, unlit_cube()) else {
        return;
    };
    let unlit = fragment_of(PLUS_Y, UNLIT, true);
    let want = shown(unlit, HALF, 1.0, "unlit");
    // Both faces alike, whatever the light.
    for (point, what) in [(ON_PLUS_Z, "+z"), (ON_PLUS_Y, "+y")] {
        assert_near(&pixels, point, want, what);
    }
    assert!(!near(want, over(BACKDROP, unlit)), "mixed");
    let shaded = colorized(fragment_of(PLUS_Y, UNLIT, false), HALF);
    assert!(!near(want, over(BACKDROP, shaded)), "not as a lit face");
}
