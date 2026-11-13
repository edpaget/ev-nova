//! The camera and the light 3D models ([`nova_view::DrawCommand::Model`])
//! are drawn with, worked out here so the adapter only multiplies.
//!
//! # The camera
//!
//! Orthographic, in logical units: a model-space point `p` lands at
//! logical `centre + scale · R · p`, R the pose's turn
//! ([`nova_view::Orientation::matrix`]), x right and y down as sprites
//! are placed. The projection onto clip space is the sprites' (x from −1
//! at the logical space's left edge to 1 at its right, y from 1 at its
//! top to −1 at its bottom), so drawn onto the same viewport a model
//! centred at (x, y) is centred where a sprite centred at (x, y) is, at
//! the same scale, in every world space and every widened view.
//!
//! The third axis, toward the camera, becomes depth: 0 is nearest, and
//! [`FAR`] is the furthest any model reaches, short of the depth buffer's
//! clear value, 1. Depth resolves a model against itself alone. Each
//! model of a batch takes a band of that range of its own
//! ([`band`]), each later one nearer than every earlier one, so models
//! layer in the draw list's order, as sprites do, and the depth test
//! hides only a model's own far faces.
//!
//! A part mounted on a model ([`nova_view::DrawCommand::Mounted`], an
//! outfit on its hull) is the exception: it lands at the model's pose
//! composed with its own place and turn ([`part`]), and shares the
//! model's band, so the two hide each other by their 3D positions. The
//! model and its parts take their depth over the furthest any of them
//! reaches from the model's origin ([`reach`]), so no part reaches out of
//! the band.
//!
//! # The light
//!
//! One directional light and an ambient term, Lambert's, in the gamma
//! space every canvas blends in ([`shade`]). The shader computes the same
//! formula per fragment from the interpolated normal.
//!
//! # A fragment
//!
//! [`fragment`] is the whole of what a model's fragment shows: its vertex
//! colour times its texture's texel, lit unless its material is unlit.
//! A model drawn colorized (the command's colorize: a ship's murk, its
//! ion tint, a gate's wash) shows [`colorized`] of it: its colour mixed
//! toward the colorize's as a sprite's texel is, on 5-bit channels. A
//! model drawn faded ([`ModelInstance::alpha`], the command's alpha as
//! [`alpha`] takes it) shows [`faded`] of that: its alpha times the
//! model's, blended straight over what is beneath. So the order is
//! `faded(colorized(fragment(..), colorize), alpha)`:
//!
//! - the colorize after the light, since a sprite colorizes the texel it
//!   would otherwise show and a model would otherwise show its lit
//!   fragment, and so an unlit fragment the same;
//! - the alpha after the colorize, as a sprite's tint applies to its
//!   mixed texel; the mix keeps the fragment's own alpha.
//!
//! `models.wgsl` mirrors all three.

use nova_view::{Color, Colorize, ModelPose, Mounting};

use crate::gpu::ModelInstance;
use crate::model::MeshId;
use crate::viewport::LogicalSize;

/// The depth of the furthest point any model reaches: short of 1, the
/// depth the buffer is cleared to, so no drawn point fails a less-than
/// test against the clear.
pub const FAR: f32 = 0.999;

/// A directional light with an ambient term.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Toward the light, on the view's axes (x right, y down, z toward
    /// the camera). [`shade`] normalises it.
    pub direction: [f32; 3],
    /// The fraction of a surface's colour it shows facing away from the
    /// light, 0 to 1.
    pub ambient: f32,
}

impl Light {
    /// The light every models batch is drawn with: from up and to the
    /// left, in front of the screen, `(−1, −1, 2)` normalised, with a
    /// quarter ambient.
    pub const STANDARD: Self = Self {
        direction: [-0.408_248_3, -0.408_248_3, 0.816_496_6],
        ambient: 0.25,
    };
}

/// `base`'s colour lit by `light` on a surface facing `normal` (on the
/// view's axes; either need not be a unit vector): each colour channel
/// times `ambient + (1 − ambient) · max(0, n · l)`, n and l normalised, and
/// the alpha kept. A zero normal faces no way, so it is lit by the
/// ambient alone.
#[must_use]
pub fn shade(normal: [f32; 3], base: [f32; 4], light: Light) -> [f32; 4] {
    let facing = dot(unit(normal), unit(light.direction)).max(0.0);
    let lit = light.ambient + (1.0 - light.ambient) * facing;
    let [red, green, blue, alpha] = base;
    [red * lit, green * lit, blue * lit, alpha]
}

/// One fragment of a model: its vertex `color` times its section's
/// `texel` (white where the section has no texture), each of the four
/// channels, then lit by `light` on a surface facing `normal` ([`shade`])
/// unless it is `unlit`, which draws the product as it is whatever the
/// normal and the light.
#[must_use]
pub fn fragment(
    normal: [f32; 3],
    color: [f32; 4],
    texel: [f32; 4],
    unlit: bool,
    light: Light,
) -> [f32; 4] {
    let base = [0, 1, 2, 3].map(|channel| color[channel] * texel[channel]);
    if unlit {
        base
    } else {
        shade(normal, base, light)
    }
}

/// The instance that draws `mesh`, `radius` model units across at most
/// from its origin ([`crate::model::Mesh::radius`]), at `pose` in the
/// `logical` space, its depth over the whole range from 0 to [`FAR`]
/// until [`band`] gives it its band. `None` when it draws nothing: a
/// scale that is not a positive, finite number, an orientation that
/// does not normalise, a centre that is not finite, or a radius that is
/// not a positive, finite number (a mesh with no extent).
#[must_use]
pub fn instance(
    logical: LogicalSize,
    pose: &ModelPose,
    mesh: MeshId,
    radius: f32,
) -> Option<ModelInstance> {
    let still = Mounting {
        at: [0.0; 3],
        turn: 0.0,
    };
    placed(logical, pose, &still, mesh, radius, false)
}

/// The instance that draws `mesh` as a part mounted on a model at `pose`
/// ([`nova_view::DrawCommand::Mounted`]): its point `p` lands where the
/// model's point `mounting.at + R_z(mounting.turn) · p` does, its depth
/// spanning 0 to [`FAR`] over `reach` model units from the model's origin
/// (the group's [`reach`]), and it shares the model's band ([`band`]).
/// `None` when it draws nothing: a pose [`instance`] refuses, a reach
/// that is not a positive, finite number, or an `at` or a turn that is
/// not finite.
///
/// The model drawn with its parts takes the same reach: its
/// [`instance`] with `reach` as its radius.
#[must_use]
pub fn part(
    logical: LogicalSize,
    pose: &ModelPose,
    mounting: &Mounting,
    mesh: MeshId,
    reach: f32,
) -> Option<ModelInstance> {
    placed(logical, pose, mounting, mesh, reach, true)
}

/// How far from a model's origin it and the parts mounted on it reach,
/// in its units: its own `radius`, or a part's `at`'s distance plus that
/// part's radius, whichever is furthest. Their depths are scaled over it,
/// so no part reaches out of the range ([`part`]).
#[must_use]
pub fn reach(radius: f32, parts: impl IntoIterator<Item = (Mounting, f32)>) -> f32 {
    parts.into_iter().fold(radius, |reach, (mounting, part)| {
        reach.max(dot(mounting.at, mounting.at).sqrt() + part)
    })
}

/// [`instance`] and [`part`]: `mesh` placed on the model at `pose` as
/// `mounting` says, its depth over `reach`, sharing the band before it
/// when `shares_band`.
fn placed(
    logical: LogicalSize,
    pose: &ModelPose,
    mounting: &Mounting,
    mesh: MeshId,
    reach: f32,
    shares_band: bool,
) -> Option<ModelInstance> {
    let positive = |v: f32| v.is_finite() && v > 0.0;
    if !positive(pose.scale)
        || !positive(reach)
        || !pose.center.x.is_finite()
        || !pose.center.y.is_finite()
        || !mounting.turn.is_finite()
        || !mounting.at.iter().all(|c| c.is_finite())
    {
        return None;
    }
    let hull = pose.orientation.matrix()?;
    // The part's turn about z, then the hull's: R · R_z.
    let (sin, cos) = mounting.turn.sin_cos();
    let about_z = [[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]];
    let r = hull.map(|row| [0, 1, 2].map(|col| dot(row, about_z.map(|turn| turn[col]))));
    // Where the part's origin lands from the hull's, on the view's axes.
    let shift: [f32; 3] = hull.map(|row| dot(row, mounting.at));
    let (w, h) = (logical.w as f32, logical.h as f32);
    let (sx, sy) = (2.0 * pose.scale / w, -2.0 * pose.scale / h);
    let k = -FAR / (2.0 * reach);
    let rows = [
        [
            sx * r[0][0],
            sx * r[0][1],
            sx * r[0][2],
            2.0 * pose.center.x / w - 1.0 + sx * shift[0],
        ],
        [
            sy * r[1][0],
            sy * r[1][1],
            sy * r[1][2],
            1.0 - 2.0 * pose.center.y / h + sy * shift[1],
        ],
        [
            k * r[2][0],
            k * r[2][1],
            k * r[2][2],
            FAR / 2.0 + k * shift[2],
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    Some(ModelInstance {
        mesh,
        clip_from_model: [0, 1, 2, 3].map(|col| rows.map(|row| row[col])),
        normal_from_model: [0, 1, 2].map(|col| r.map(|row| row[col])),
        shares_band,
        alpha: 1.0,
    })
}

/// The alpha a model drawn at `alpha` ([`nova_view::DrawCommand::Model`])
/// draws at: `alpha` held to 1, or `None`, drawing nothing, when it isn't
/// finite or is 0 or less.
#[must_use]
pub fn alpha(alpha: f32) -> Option<f32> {
    (alpha.is_finite() && alpha > 0.0).then(|| alpha.min(1.0))
}

/// A fragment of a model drawn at `alpha` (its [`ModelInstance::alpha`]):
/// `fragment`, as [`fragment`] gives it, with its alpha times `alpha` and
/// its colour as it is, blended straight over what is beneath. The
/// adapter draws only the model's nearest surface so (see the `models`
/// module), so nothing of the model shows through itself.
#[must_use]
pub fn faded(fragment: [f32; 4], alpha: f32) -> [f32; 4] {
    let [red, green, blue, own] = fragment;
    [red, green, blue, own * alpha]
}

/// A fragment of a model drawn colorized (its
/// [`ModelInstance::colorize`]): `fragment`, as [`fragment`] gives it,
/// mixed toward `colorize`'s colour as a sprite's texel is, its alpha
/// kept. Each colour channel is held to 0 to 1, rounded to its nearest
/// 8-bit value and narrowed to its top 5 bits, mixed
/// ([`Colorize::mix`]: the kept share and the colour's each floored, then
/// OR-ed), and widened back by bit replication
/// ([`nova_view::Color::from_rgb555`]). A level of 32 is the colour
/// exactly, whatever the fragment and so whatever the light. A colorize
/// that doesn't mix ([`Colorize::mixes`]) leaves the fragment as it is,
/// unnarrowed.
///
/// `shader.wgsl` mixes a sprite's texel so (`texel_mixed`), and
/// `models.wgsl` mirrors this.
#[must_use]
pub fn colorized(fragment: [f32; 4], colorize: Colorize) -> [f32; 4] {
    if !colorize.mixes() {
        return fragment;
    }
    let [red, green, blue, alpha] = fragment;
    let s5 = [red, green, blue].map(|channel| {
        // Held to 0 to 255 before it is cast. Halves round to even, as
        // WGSL's `round` does.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let c8 = (channel.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
        c8 >> 3
    });
    let mixed = Color::from_rgb555(colorize.mix(s5));
    let [red, green, blue] = [mixed.r, mixed.g, mixed.b].map(|channel| f32::from(channel) / 255.0);
    [red, green, blue, alpha]
}

/// Gives each of `models`, one batch's in draw order, its band of the
/// depth range: the i-th of n bands takes `[FAR · (n − i − 1) / n,
/// FAR · (n − i) / n]`, so each later band is nearer than every point of
/// every earlier one. A model takes a band of its own, and each part
/// after it ([`ModelInstance::shares_band`]) takes the same band, so a
/// model and its parts depth-test against each other (one that shares
/// with nothing before it, first in the batch, takes its own). Each
/// instance's depth is taken to span 0 to [`FAR`], as [`instance`] and
/// [`part`] make it.
pub fn band(models: &mut [ModelInstance]) {
    let starts = |i: usize, model: &ModelInstance| i == 0 || !model.shares_band;
    let n = models
        .iter()
        .enumerate()
        .filter(|&(i, model)| starts(i, model))
        .count() as f32;
    let mut i = -1.0;
    for (at, model) in models.iter_mut().enumerate() {
        if starts(at, model) {
            i += 1.0;
        }
        let near = FAR * (n - i - 1.0) / n;
        let width = FAR / n;
        let m = &mut model.clip_from_model;
        for column in m.iter_mut() {
            // Depth is row 2; row 3 is (0, 0, 0, 1).
            column[2] = column[2] * width / FAR + near * column[3];
        }
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `v` scaled to a unit vector. One with no length, or no finite one,
/// gives NaNs, which [`shade`]'s `max(0.0)` takes as facing away
/// (`f32::max` returns the other operand of a NaN), as the shader's
/// `select` does for a zero normal.
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    v.map(|c| c / length)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use nova_view::{Colorize, Mounting, Orientation, Point};

    use super::*;

    const LOGICAL: LogicalSize = LogicalSize { w: 32, h: 24 };
    const MESH: MeshId = MeshId(4);

    fn pose(x: f32, y: f32, orientation: Orientation, scale: f32) -> ModelPose {
        ModelPose {
            center: Point::new(x, y),
            orientation,
            scale,
        }
    }

    /// `p` in model units through `instance`'s `clip_from_model`.
    fn clip(instance: &ModelInstance, p: [f32; 3]) -> [f32; 4] {
        let m = instance.clip_from_model;
        let p = [p[0], p[1], p[2], 1.0];
        [0, 1, 2, 3].map(|row| (0..4).map(|col| m[col][row] * p[col]).sum())
    }

    /// `n` through `instance`'s `normal_from_model`.
    fn normal(instance: &ModelInstance, n: [f32; 3]) -> [f32; 3] {
        let m = instance.normal_from_model;
        [0, 1, 2].map(|row| (0..3).map(|col| m[col][row] * n[col]).sum())
    }

    /// The logical point a clip-space position projects onto, by the
    /// sprites' projection undone: `x = (cx + 1) / 2 · w`,
    /// `y = (1 − cy) / 2 · h`.
    fn logical_of(logical: LogicalSize, c: [f32; 4]) -> (f32, f32) {
        (
            f32::midpoint(c[0], 1.0) * logical.w as f32,
            (1.0 - c[1]) / 2.0 * logical.h as f32,
        )
    }

    #[track_caller]
    fn assert_near(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-5, "{a} is not {b}");
    }

    fn made(logical: LogicalSize, pose: &ModelPose) -> ModelInstance {
        instance(logical, pose, MESH, 1.0).expect("drawable")
    }

    #[test]
    fn an_instance_and_a_part_draw_whole_until_faded() {
        let pose = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        assert_eq!(made(LOGICAL, &pose).alpha, 1.0);
        let still = Mounting {
            at: [0.0; 3],
            turn: 0.0,
        };
        let part = part(LOGICAL, &pose, &still, MESH, 1.0).expect("drawable");
        assert_eq!(part.alpha, 1.0);
    }

    #[test]
    fn a_drawable_alpha_is_finite_and_above_0_and_held_to_1() {
        for drawn in [0.25, 1.0, f32::MIN_POSITIVE] {
            assert_eq!(alpha(drawn), Some(drawn), "{drawn}");
        }
        assert_eq!(alpha(1.5), Some(1.0));
        assert_eq!(alpha(f32::MAX), Some(1.0));
        for refused in [0.0, -0.0, -0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(alpha(refused), None, "{refused}");
        }
    }

    #[test]
    fn a_faded_fragment_keeps_its_colour_and_multiplies_its_alpha() {
        assert_eq!(
            faded([0.5, 0.25, 1.0, 0.8], 0.5),
            [0.5, 0.25, 1.0, 0.8 * 0.5]
        );
        assert_eq!(faded([0.5, 0.25, 1.0, 0.8], 1.0), [0.5, 0.25, 1.0, 0.8]);
    }

    #[test]
    fn a_models_origin_lands_where_a_sprite_centred_there_does() {
        // (12, 6) of 32 x 24: the sprites' projection puts it at clip
        // x = 12 / 32 · 2 − 1 = −0.25, y = 1 − 6 / 24 · 2 = 0.5.
        let at = made(LOGICAL, &pose(12.0, 6.0, Orientation::IDENTITY, 5.0));
        let origin = clip(&at, [0.0; 3]);
        assert_near(origin[0], -0.25);
        assert_near(origin[1], 0.5);
        assert_eq!(origin[3], 1.0, "no perspective");
        assert_eq!(at.mesh, MESH);
        // Turned, the origin stays put.
        let turned = made(
            LOGICAL,
            &pose(12.0, 6.0, Orientation::about([1.0, 2.0, 3.0], 1.0), 5.0),
        );
        let origin = clip(&turned, [0.0; 3]);
        assert_near(origin[0], -0.25);
        assert_near(origin[1], 0.5);
    }

    #[test]
    fn one_model_unit_spans_scale_logical_units() {
        let at = made(LOGICAL, &pose(12.0, 6.0, Orientation::IDENTITY, 5.0));
        let (x, y) = logical_of(LOGICAL, clip(&at, [1.0, 0.0, 0.0]));
        assert_near(x, 17.0);
        assert_near(y, 6.0);
        let (x, y) = logical_of(LOGICAL, clip(&at, [0.0, 1.0, 0.0]));
        assert_near(x, 12.0);
        assert_near(y, 11.0);
        let (x, y) = logical_of(LOGICAL, clip(&at, [-0.5, -0.5, 0.0]));
        assert_near(x, 9.5);
        assert_near(y, 3.5);
    }

    #[test]
    fn a_widened_logical_space_keeps_units_square_in_logical_terms() {
        let wide = LogicalSize { w: 1365, h: 768 };
        let at = made(wide, &pose(600.0, 400.0, Orientation::IDENTITY, 20.0));
        let (x, y) = logical_of(wide, clip(&at, [1.0, 1.0, 0.0]));
        assert!((x - 620.0).abs() < 1e-3, "{x}");
        assert!((y - 420.0).abs() < 1e-3, "{y}");
    }

    #[test]
    fn orientation_turns_points_and_normals_alike() {
        let quarter = Orientation::about([0.0, 0.0, 1.0], FRAC_PI_2);
        let at = made(LOGICAL, &pose(12.0, 6.0, quarter, 5.0));
        // +x turns to +y: five units below the centre.
        let (x, y) = logical_of(LOGICAL, clip(&at, [1.0, 0.0, 0.0]));
        assert_near(x, 12.0);
        assert_near(y, 11.0);
        let n = normal(&at, [1.0, 0.0, 0.0]);
        assert_near(n[0], 0.0);
        assert_near(n[1], 1.0);
        assert_near(n[2], 0.0);
        // About x, +y turns toward the camera, nearer.
        let tilted = made(
            LOGICAL,
            &pose(
                12.0,
                6.0,
                Orientation::about([1.0, 0.0, 0.0], FRAC_PI_2),
                5.0,
            ),
        );
        let n = normal(&tilted, [0.0, 1.0, 0.0]);
        assert_near(n[2], 1.0);
        assert!(clip(&tilted, [0.0, 0.5, 0.0])[2] < clip(&tilted, [0.0, -0.5, 0.0])[2]);
        // The normals ignore the scale and the place.
        let elsewhere = made(LOGICAL, &pose(1.0, 2.0, quarter, 9.0));
        assert_eq!(elsewhere.normal_from_model, at.normal_from_model);
    }

    #[test]
    fn a_turned_point_lands_at_the_centre_plus_scale_times_its_turn() {
        let orientation = Orientation::about([0.3, -1.0, 0.7], 2.1);
        let at = made(LOGICAL, &pose(12.0, 6.0, orientation, 5.0));
        for p in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.4, -0.7, 0.9],
        ] {
            let turned = orientation.rotate(p).expect("a unit turn");
            let (x, y) = logical_of(LOGICAL, clip(&at, p));
            assert_near(x, 12.0 + 5.0 * turned[0]);
            assert_near(y, 6.0 + 5.0 * turned[1]);
            // Depth from the turned point's third axis alone.
            assert_near(clip(&at, p)[2], FAR * (0.5 - turned[2] / 2.0));
            let n = normal(&at, p);
            for (got, want) in n.iter().zip(turned) {
                assert_near(*got, want);
            }
        }
    }

    #[test]
    fn nearer_the_camera_is_shallower_depth() {
        let at = made(LOGICAL, &pose(12.0, 6.0, Orientation::IDENTITY, 5.0));
        let near = clip(&at, [0.0, 0.0, 1.0])[2];
        let middle = clip(&at, [0.0, 0.0, 0.0])[2];
        let far = clip(&at, [0.0, 0.0, -1.0])[2];
        assert!(near < middle && middle < far, "{near} {middle} {far}");
        // A radius out each way spans the whole range.
        assert_near(near, 0.0);
        assert_near(far, FAR);
        // x and y don't move the depth.
        assert_eq!(clip(&at, [3.0, -2.0, 0.0])[2], middle);
    }

    #[test]
    fn the_depth_is_scaled_by_the_meshs_radius_not_the_pose() {
        let at = instance(
            LOGICAL,
            &pose(12.0, 6.0, Orientation::IDENTITY, 5.0),
            MESH,
            4.0,
        )
        .expect("drawable");
        assert_near(clip(&at, [0.0, 0.0, 4.0])[2], 0.0);
        assert_near(clip(&at, [0.0, 0.0, -4.0])[2], FAR);
    }

    /// The depths of a radius-1 mesh's nearest and furthest points.
    fn depth_span(instance: &ModelInstance) -> (f32, f32) {
        let [a, b] = [1.0, -1.0].map(|z| clip(instance, [0.0, 0.0, z])[2]);
        (a.min(b), a.max(b))
    }

    #[test]
    fn every_depth_of_a_later_band_is_nearer_than_any_of_an_earlier_one() {
        let at = made(LOGICAL, &pose(12.0, 6.0, Orientation::IDENTITY, 5.0));
        let mut models = vec![at; 3];
        band(&mut models);
        let spans: Vec<(f32, f32)> = models.iter().map(depth_span).collect();
        for pair in spans.windows(2) {
            let (earlier, later) = (pair[0], pair[1]);
            assert!(later.1 <= earlier.0 + 1e-6, "{spans:?}");
            assert!(later.0 < later.1, "{spans:?}");
        }
        assert_near(spans[0].1, FAR);
        assert_near(spans[2].0, 0.0);
        assert_near(spans[1].0, FAR / 3.0);
        assert_near(spans[1].1, FAR * 2.0 / 3.0);
        // Banding leaves x and y as they were.
        for model in &models {
            assert_eq!(
                clip(model, [1.0, 2.0, 0.0])[..2],
                clip(&at, [1.0, 2.0, 0.0])[..2]
            );
            assert_eq!(model.normal_from_model, at.normal_from_model);
        }
    }

    #[test]
    fn one_model_alone_keeps_the_whole_range() {
        let at = made(LOGICAL, &pose(12.0, 6.0, Orientation::IDENTITY, 5.0));
        let mut models = vec![at];
        band(&mut models);
        assert_eq!(depth_span(&models[0]), depth_span(&at));
        band(&mut []);
    }

    #[test]
    fn depth_stays_inside_zero_to_one_for_a_unit_mesh_at_any_orientation() {
        let corners: Vec<[f32; 3]> = (0..8)
            .map(|bits| [0, 1, 2].map(|axis| if bits >> axis & 1 == 1 { 0.5 } else { -0.5 }))
            .collect();
        let radius = 3.0_f32.sqrt() / 2.0;
        for step in 0..24 {
            let angle = step as f32 * PI / 12.0;
            for axis in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [-2.0, 0.5, 1.0],
            ] {
                let orientation = Orientation::about(axis, angle);
                let at = instance(LOGICAL, &pose(5.0, 5.0, orientation, 3.0), MESH, radius)
                    .expect("drawable");
                let mut banded = vec![at; 2];
                band(&mut banded);
                for model in [&at, &banded[0], &banded[1]] {
                    for &corner in &corners {
                        let depth = clip(model, corner)[2];
                        assert!(
                            (-1e-6..=FAR + 1e-6).contains(&depth),
                            "{depth} at {angle} about {axis:?}"
                        );
                    }
                }
            }
        }
        const { assert!(FAR < 1.0) };
    }

    #[test]
    fn a_model_with_no_drawable_scale_orientation_centre_or_radius_has_no_instance() {
        let good = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        assert!(instance(LOGICAL, &good, MESH, 1.0).is_some());
        for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let bad = ModelPose { scale, ..good };
            assert_eq!(instance(LOGICAL, &bad, MESH, 1.0), None, "{scale}");
        }
        let zero = Orientation {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let nan = Orientation {
            x: f32::NAN,
            ..Orientation::IDENTITY
        };
        for orientation in [zero, nan] {
            let bad = ModelPose {
                orientation,
                ..good
            };
            assert_eq!(instance(LOGICAL, &bad, MESH, 1.0), None);
        }
        for center in [Point::new(f32::NAN, 1.0), Point::new(1.0, f32::INFINITY)] {
            let bad = ModelPose { center, ..good };
            assert_eq!(instance(LOGICAL, &bad, MESH, 1.0), None);
        }
        for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(instance(LOGICAL, &good, MESH, radius), None, "{radius}");
        }
    }

    #[test]
    fn an_unnormalised_orientation_draws_as_its_unit_one() {
        let quarter = Orientation::about([0.0, 0.0, 1.0], FRAC_PI_2);
        let doubled = Orientation {
            w: quarter.w * 2.0,
            x: 0.0,
            y: 0.0,
            z: quarter.z * 2.0,
        };
        let a = made(LOGICAL, &pose(1.0, 2.0, quarter, 3.0));
        let b = made(LOGICAL, &pose(1.0, 2.0, doubled, 3.0));
        for (x, y) in a
            .clip_from_model
            .iter()
            .flatten()
            .zip(b.clip_from_model.iter().flatten())
        {
            assert_near(*x, *y);
        }
    }

    /// A turn of `turn` about z by Orientation's own rule, knowing nothing
    /// of the camera.
    fn about_z(turn: f32, p: [f32; 3]) -> [f32; 3] {
        Orientation::about([0.0, 0.0, 1.0], turn)
            .rotate(p)
            .expect("a unit turn")
    }

    fn mounting(at: [f32; 3], turn: f32) -> Mounting {
        Mounting { at, turn }
    }

    #[test]
    fn a_part_lands_where_its_model_s_pose_puts_at_plus_its_turned_point() {
        let orientation = Orientation::about([0.3, -1.0, 0.7], 2.1);
        let hull = pose(12.0, 6.0, orientation, 5.0);
        let on = mounting([1.0, -2.0, 0.5], 0.7);
        let reach = 4.0;
        let at = part(LOGICAL, &hull, &on, MESH, reach).expect("drawable");
        assert_eq!(at.mesh, MESH);
        for p in [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.4, -0.7, 0.9],
        ] {
            let local = about_z(0.7, p);
            let on_hull = [0, 1, 2].map(|k| on.at[k] + local[k]);
            let turned = orientation.rotate(on_hull).expect("a unit turn");
            let (x, y) = logical_of(LOGICAL, clip(&at, p));
            assert!((x - (12.0 + 5.0 * turned[0])).abs() < 1e-4, "{p:?}: {x}");
            assert!((y - (6.0 + 5.0 * turned[1])).abs() < 1e-4, "{p:?}: {y}");
            // Depth over the group's reach, from the hull's middle.
            let depth = clip(&at, p)[2];
            assert!((depth - FAR * (0.5 - turned[2] / (2.0 * reach))).abs() < 1e-5);
            // Normals turn by the part's turn, then the hull's.
            let n = normal(&at, p);
            let want = orientation.rotate(local).expect("a unit turn");
            for (got, want) in n.iter().zip(want) {
                assert_near(*got, want);
            }
        }
    }

    #[test]
    fn a_hull_s_own_instance_at_a_reach_is_its_instance_with_that_radius() {
        let hull = pose(12.0, 6.0, Orientation::about([1.0, 2.0, 3.0], 1.0), 5.0);
        let still = mounting([0.0; 3], 0.0);
        let mut alone = instance(LOGICAL, &hull, MESH, 3.0).expect("drawable");
        alone.shares_band = true;
        assert_eq!(part(LOGICAL, &hull, &still, MESH, 3.0), Some(alone));
        assert!(!instance(LOGICAL, &hull, MESH, 3.0).unwrap().shares_band);
    }

    #[test]
    fn a_part_shares_its_model_s_band() {
        let hull = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        let at = part(LOGICAL, &hull, &mounting([0.0; 3], 0.0), MESH, 1.0);
        assert!(at.expect("drawable").shares_band);
    }

    #[test]
    fn a_part_at_or_turned_by_what_is_not_finite_or_on_a_bad_pose_has_no_instance() {
        let good = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for at in [[bad, 0.0, 0.0], [0.0, bad, 0.0], [0.0, 0.0, bad]] {
                assert_eq!(part(LOGICAL, &good, &mounting(at, 0.0), MESH, 2.0), None);
            }
            assert_eq!(
                part(LOGICAL, &good, &mounting([0.0; 3], bad), MESH, 2.0),
                None
            );
        }
        let still = mounting([0.0; 3], 0.0);
        for reach in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(part(LOGICAL, &good, &still, MESH, reach), None);
        }
        let flat = ModelPose { scale: 0.0, ..good };
        assert_eq!(part(LOGICAL, &flat, &still, MESH, 2.0), None);
    }

    #[test]
    fn a_group_reaches_as_far_as_its_hull_or_its_furthest_part() {
        assert_eq!(reach(2.0, []), 2.0);
        // |(3, 4, 0)| = 5, and the part reaches 0.5 past it.
        let far = (mounting([3.0, 4.0, 0.0], 1.0), 0.5);
        let near = (mounting([0.0, 0.0, 1.0], 0.0), 0.25);
        assert_eq!(reach(2.0, [near]), 2.0);
        assert_eq!(reach(2.0, [near, far]), 5.5);
        assert_eq!(reach(9.0, [near, far]), 9.0);
    }

    #[test]
    fn a_part_reaching_past_its_hull_keeps_every_point_of_both_inside_the_range() {
        let orientation = Orientation::about([1.0, 0.3, -0.2], 0.9);
        let hull = pose(12.0, 6.0, orientation, 5.0);
        let on = mounting([0.0, 0.0, 1.5], 0.4);
        // A unit cube hull, radius √3/2, and a cube part of the same size
        // standing high on it.
        let radius = 3.0_f32.sqrt() / 2.0;
        let r = reach(radius, [(on, radius)]);
        assert!(r > radius);
        let corners: Vec<[f32; 3]> = (0..8)
            .map(|bits| [0, 1, 2].map(|axis| if bits >> axis & 1 == 1 { 0.5 } else { -0.5 }))
            .collect();
        let hull_at = instance(LOGICAL, &hull, MESH, r).expect("drawable");
        let part_at = part(LOGICAL, &hull, &on, MESH, r).expect("drawable");
        let mut banded = vec![hull_at, part_at];
        band(&mut banded);
        for model in [&hull_at, &part_at, &banded[0], &banded[1]] {
            for &corner in &corners {
                let depth = clip(model, corner)[2];
                assert!((-1e-6..=FAR + 1e-6).contains(&depth), "{depth}");
            }
        }
        // Out of range at the hull's own radius: why the hull is rescaled.
        let unscaled = part(LOGICAL, &hull, &on, MESH, radius).expect("drawable");
        assert!(
            corners
                .iter()
                .any(|&c| !(0.0..=FAR).contains(&clip(&unscaled, c)[2]))
        );
    }

    #[test]
    fn a_hull_and_its_parts_take_one_band_and_the_next_hull_another() {
        let hull = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        let a = instance(LOGICAL, &hull, MESH, 1.0).expect("drawable");
        let on = part(LOGICAL, &hull, &mounting([0.0; 3], 0.0), MESH, 1.0).expect("drawable");
        let mut models = vec![a, on, on, a];
        band(&mut models);
        let spans: Vec<(f32, f32)> = models.iter().map(depth_span).collect();
        assert_eq!(spans[0], spans[1]);
        assert_eq!(spans[1], spans[2]);
        assert_near(spans[0].1, FAR);
        assert_near(spans[0].0, FAR / 2.0);
        assert_near(spans[3].1, FAR / 2.0);
        assert_near(spans[3].0, 0.0);
    }

    #[test]
    fn a_part_first_in_a_batch_still_takes_a_band_of_its_own() {
        let hull = pose(12.0, 6.0, Orientation::IDENTITY, 5.0);
        let on = part(LOGICAL, &hull, &mounting([0.0; 3], 0.0), MESH, 1.0).expect("drawable");
        let mut models = vec![on];
        band(&mut models);
        assert_eq!(depth_span(&models[0]), depth_span(&on));
    }

    const RED: [f32; 4] = [1.0, 0.5, 0.25, 0.75];

    #[test]
    fn shade_gives_full_light_facing_the_light_and_ambient_facing_away() {
        let light = Light::STANDARD;
        let toward = light.direction;
        let away = toward.map(|c| -c);
        assert_eq!(
            shade(toward, RED, light).map(|c| (c * 1e4).round()),
            RED.map(|c| (c * 1e4).round())
        );
        let ambient = shade(away, RED, light);
        assert_near(ambient[0], 0.25);
        assert_near(ambient[1], 0.125);
        // Edge-on to the light is ambient too.
        let edge_on = [toward[1], -toward[0], 0.0];
        assert_near(shade(edge_on, RED, light)[0], 0.25);
        // Half way: ambient plus half the rest.
        let light = Light {
            direction: [0.0, 0.0, 2.0],
            ambient: 0.2,
        };
        let sixty = [(PI / 3.0).sin(), 0.0, (PI / 3.0).cos()];
        assert_near(shade(sixty, RED, light)[0], 0.2 + 0.8 * 0.5);
        // A zero normal is lit by the ambient alone.
        assert_near(shade([0.0; 3], RED, light)[0], 0.2);
    }

    #[test]
    fn shade_lights_a_surface_facing_the_light_along_each_axis_fully() {
        for axis in 0..3 {
            let mut toward = [0.0; 3];
            toward[axis] = 2.0;
            let light = Light {
                direction: toward,
                ambient: 0.5,
            };
            assert_eq!(shade(toward, RED, light), RED, "axis {axis}");
            let away = toward.map(|c| -c);
            assert_eq!(shade(away, RED, light), [0.5, 0.25, 0.125, 0.75]);
        }
        // Not finite, a normal faces no way.
        let light = Light {
            direction: [0.0, 0.0, 1.0],
            ambient: 0.5,
        };
        assert_eq!(shade([0.0, 0.0, f32::INFINITY], RED, light)[0], 0.5);
        assert_eq!(shade([f32::NAN, 0.0, 1.0], RED, light)[0], 0.5);
    }

    #[test]
    fn shade_scales_the_base_colour_and_keeps_alpha() {
        let light = Light {
            direction: [0.0, 0.0, 1.0],
            ambient: 0.5,
        };
        // Facing straight at it, a normal of any length: everything lit.
        let lit = shade([0.0, 0.0, 3.0], RED, light);
        assert_eq!(lit, RED);
        let dim = shade([0.0, 0.0, -1.0], RED, light);
        assert_eq!(dim, [0.5, 0.25, 0.125, 0.75]);
    }

    /// A texel apart from white in every channel.
    const TEXEL: [f32; 4] = [0.5, 0.25, 0.75, 0.5];
    const WHITE: [f32; 4] = [1.0; 4];

    /// Normals and lights apart: facing the light, away from it, edge-on,
    /// and none at all.
    fn normals_and_lights() -> Vec<([f32; 3], Light)> {
        let lights = [
            Light::STANDARD,
            Light {
                direction: [0.0, 0.0, 1.0],
                ambient: 0.5,
            },
            Light {
                direction: [3.0, -1.0, 0.5],
                ambient: 0.0,
            },
        ];
        let normals = [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [-0.4, 0.8, 0.2],
            [0.0; 3],
        ];
        lights
            .into_iter()
            .flat_map(|light| normals.map(|normal| (normal, light)))
            .collect()
    }

    #[test]
    fn an_unlit_fragment_is_its_colour_times_its_texel_whatever_the_light() {
        let product = [0.5, 0.125, 0.1875, 0.375];
        for (normal, light) in normals_and_lights() {
            assert_eq!(fragment(normal, RED, TEXEL, true, light), product);
            // With a white texel, its vertex colour exactly.
            assert_eq!(fragment(normal, RED, WHITE, true, light), RED);
            // With a white vertex, its texel exactly.
            assert_eq!(fragment(normal, WHITE, TEXEL, true, light), TEXEL);
        }
    }

    #[test]
    fn a_lit_fragment_is_the_product_shaded() {
        let product = [0.5, 0.125, 0.1875, 0.375];
        for (normal, light) in normals_and_lights() {
            assert_eq!(
                fragment(normal, RED, TEXEL, false, light),
                shade(normal, product, light)
            );
            // With a white texel, a model with no texture as it was lit.
            assert_eq!(
                fragment(normal, RED, WHITE, false, light),
                shade(normal, RED, light)
            );
        }
        // Lit and unlit differ wherever the light is not full.
        let dim = Light {
            direction: [0.0, 0.0, 1.0],
            ambient: 0.5,
        };
        let away = [0.0, 0.0, -1.0];
        assert_eq!(
            fragment(away, RED, TEXEL, false, dim),
            [0.25, 0.0625, 0.09375, 0.375]
        );
        assert_ne!(
            fragment(away, RED, TEXEL, false, dim),
            fragment(away, RED, TEXEL, true, dim)
        );
    }

    #[test]
    fn a_fragment_s_alpha_is_its_colour_s_times_its_texel_s_lit_or_not() {
        let light = Light {
            direction: [0.0, 0.0, 1.0],
            ambient: 0.5,
        };
        for unlit in [false, true] {
            let faded = fragment([0.0, 0.0, -1.0], RED, TEXEL, unlit, light);
            assert_eq!(faded[3], 0.75 * 0.5, "unlit {unlit}");
            let clear = fragment([0.0, 0.0, 1.0], RED, [1.0, 1.0, 1.0, 0.0], unlit, light);
            assert_eq!(clear[3], 0.0, "unlit {unlit}");
        }
    }

    /// A fragment whose colour sits mid-way in the 5-bit steps `s5`, at
    /// `alpha`.
    fn of_steps(s5: [u8; 3], alpha: f32) -> [f32; 4] {
        let [r, g, b] = s5.map(|c| (f32::from(c) * 8.0 + 4.0) / 255.0);
        [r, g, b, alpha]
    }

    /// The 5-bit colour `rgb5` as the fragment that shows it, at `alpha`.
    fn widened(rgb5: [u8; 3], alpha: f32) -> [f32; 4] {
        let shown = nova_view::Color::from_rgb555(rgb5);
        let [r, g, b] = [shown.r, shown.g, shown.b].map(|c| f32::from(c) / 255.0);
        [r, g, b, alpha]
    }

    #[test]
    fn a_colorized_fragment_mixes_each_5_bit_channel_and_keeps_its_alpha() {
        let tint = Colorize {
            level: 14,
            rgb5: [5, 0, 31],
        };
        let mixed = tint.mix([7, 31, 0]);
        assert_eq!(mixed, [3, 17, 13], "the sprite's arithmetic");
        assert_eq!(
            colorized(of_steps([7, 31, 0], 0.4), tint),
            widened(mixed, 0.4)
        );
        // Widened by bit replication: 3 is 24 | 0, 17 is 136 | 4, 13 is
        // 104 | 3.
        assert_eq!(
            colorized(of_steps([7, 31, 0], 0.4), tint),
            [24.0 / 255.0, 140.0 / 255.0, 107.0 / 255.0, 0.4]
        );
    }

    #[test]
    fn the_halves_are_ored_not_added() {
        let half = Colorize {
            level: 16,
            rgb5: [31, 0, 21],
        };
        // Red: 15 | 15 is 15, where adding would give 30. 15 widens to 123.
        let grey = 123.0 / 255.0;
        assert_eq!(
            colorized(of_steps([31, 31, 10], 1.0), half),
            [grey, grey, grey, 1.0]
        );
    }

    #[test]
    fn level_32_is_the_colour_whatever_the_fragment() {
        let full = Colorize {
            level: 32,
            rgb5: [3, 17, 30],
        };
        for fragment in [
            [0.0, 0.0, 0.0, 0.5],
            [1.0, 1.0, 1.0, 0.5],
            [0.9, 0.1, 0.4, 0.5],
        ] {
            assert_eq!(colorized(fragment, full), widened([3, 17, 30], 0.5));
        }
    }

    #[test]
    fn a_colorize_that_does_not_mix_leaves_the_fragment_untouched() {
        // Off every 5-bit step, so narrowing it would show.
        let fragment = [0.3, 0.7, 0.123, 0.4];
        for level in [0, 33, 200] {
            let none = Colorize {
                level,
                rgb5: [3, 17, 30],
            };
            assert_eq!(colorized(fragment, none), fragment, "{level}");
        }
        let least = Colorize {
            level: 1,
            rgb5: [3, 17, 30],
        };
        assert_ne!(colorized(fragment, least), fragment, "level 1 mixes");
    }

    #[test]
    fn a_channel_narrows_from_its_nearest_8_bit_value() {
        // At level 1 toward black a step s shows as s × 31 >> 5, so steps
        // 2, 3 and 31 show apart: 1, 2 and 30.
        let barely = Colorize {
            level: 1,
            rgb5: [0, 0, 0],
        };
        // 23.4 rounds to 23, step 2; 23.6 to 24, step 3; 247.6 to 248,
        // step 31.
        let fragment = [23.4 / 255.0, 23.6 / 255.0, 247.6 / 255.0, 1.0];
        assert_eq!(colorized(fragment, barely), widened([1, 2, 30], 1.0));
        assert_eq!(barely.mix([2, 3, 31]), [1, 2, 30]);
    }

    #[test]
    fn a_channel_outside_0_to_1_is_held_before_it_narrows() {
        let barely = Colorize {
            level: 1,
            rgb5: [0, 0, 0],
        };
        // Above 1 is step 31 and below 0 step 0, as 1 and 0 are.
        assert_eq!(
            colorized([1.5, -0.25, 300.0, 0.75], barely),
            colorized([1.0, 0.0, 1.0, 0.75], barely)
        );
        assert_eq!(
            colorized([1.5, -0.25, 300.0, 0.75], barely),
            widened([30, 0, 30], 0.75)
        );
    }

    #[test]
    fn an_unlit_fragment_is_colorized_as_a_lit_one() {
        let half = Colorize {
            level: 16,
            rgb5: [0, 4, 31],
        };
        let dim = Light {
            direction: [0.0, 0.0, 1.0],
            ambient: 0.5,
        };
        let away = [0.0, 0.0, -1.0];
        // RED × TEXEL is (0.5, 0.125, 0.1875), steps 16, 4 and 6; lit at
        // a half, (0.25, 0.0625, 0.09375), steps 8, 2 and 3.
        assert_eq!(
            colorized(fragment(away, RED, TEXEL, true, dim), half),
            widened(half.mix([16, 4, 6]), 0.375)
        );
        assert_eq!(
            colorized(fragment(away, RED, TEXEL, false, dim), half),
            widened(half.mix([8, 2, 3]), 0.375)
        );
        assert_ne!(half.mix([16, 4, 6]), half.mix([8, 2, 3]));
    }

    #[test]
    fn a_colorized_fragment_fades_by_its_alpha_after_it_mixes() {
        let half = Colorize {
            level: 16,
            rgb5: [0, 4, 31],
        };
        let [r, g, b, _] = widened(half.mix([31, 10, 7]), 1.0);
        assert_eq!(
            faded(colorized(of_steps([31, 10, 7], 0.8), half), 0.5),
            [r, g, b, 0.8 * 0.5]
        );
    }

    #[test]
    fn the_standard_light_is_a_unit_vector_from_up_left_and_in_front() {
        let [x, y, z] = Light::STANDARD.direction;
        assert_near(x * x + y * y + z * z, 1.0);
        assert!(x < 0.0 && y < 0.0 && z > 0.0);
        assert_eq!(Light::STANDARD.ambient, 0.25);
    }
}
