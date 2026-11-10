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
//! # The light
//!
//! One directional light and an ambient term, Lambert's, in the gamma
//! space every canvas blends in ([`shade`]). The shader computes the same
//! formula per fragment from the interpolated normal.

use nova_view::ModelPose;

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
    let facing = match (unit(normal), unit(light.direction)) {
        (Some(normal), Some(toward)) => dot(normal, toward).max(0.0),
        _ => 0.0,
    };
    let lit = light.ambient + (1.0 - light.ambient) * facing;
    let [red, green, blue, alpha] = base;
    [red * lit, green * lit, blue * lit, alpha]
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
    let positive = |v: f32| v.is_finite() && v > 0.0;
    if !positive(pose.scale)
        || !positive(radius)
        || !pose.center.x.is_finite()
        || !pose.center.y.is_finite()
    {
        return None;
    }
    let r = pose.orientation.matrix()?;
    let (w, h) = (logical.w as f32, logical.h as f32);
    let (sx, sy) = (2.0 * pose.scale / w, -2.0 * pose.scale / h);
    let k = -FAR / (2.0 * radius);
    let rows = [
        [
            sx * r[0][0],
            sx * r[0][1],
            sx * r[0][2],
            2.0 * pose.center.x / w - 1.0,
        ],
        [
            sy * r[1][0],
            sy * r[1][1],
            sy * r[1][2],
            1.0 - 2.0 * pose.center.y / h,
        ],
        [k * r[2][0], k * r[2][1], k * r[2][2], FAR / 2.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    Some(ModelInstance {
        mesh,
        clip_from_model: [0, 1, 2, 3].map(|col| rows.map(|row| row[col])),
        normal_from_model: [0, 1, 2].map(|col| r.map(|row| row[col])),
    })
}

/// Gives each of `models`, one batch's in draw order, its band of the
/// depth range: the i-th of n takes `[FAR · (n − i − 1) / n, FAR · (n − i)
/// / n]`, so each later model is nearer than every point of every earlier
/// one. Each instance's depth is taken to span 0 to [`FAR`], as
/// [`instance`] makes it.
pub fn band(models: &mut [ModelInstance]) {
    let n = models.len() as f32;
    for (i, model) in models.iter_mut().enumerate() {
        let i = i as f32;
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

/// `v` scaled to a unit vector, or `None` when it has no length (or no
/// finite one).
fn unit(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(v, v).sqrt();
    (length.is_finite() && length > 0.0).then(|| v.map(|c| c / length))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use nova_view::{Orientation, Point};

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

    #[test]
    fn the_standard_light_is_a_unit_vector_from_up_left_and_in_front() {
        let [x, y, z] = Light::STANDARD.direction;
        assert_near(x * x + y * y + z * z, 1.0);
        assert!(x < 0.0 && y < 0.0 && z > 0.0);
        assert_eq!(Light::STANDARD.ambient, 0.25);
    }
}
