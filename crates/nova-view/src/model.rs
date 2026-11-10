//! 3D models in the draw list: which model a [`DrawCommand::Model`]
//! draws ([`ModelKey`]) and where and how it is turned ([`ModelPose`],
//! [`Orientation`]), and where a part mounted on it sits
//! ([`DrawCommand::Mounted`], [`Mounting`]).
//!
//! # Axes
//!
//! A model's axes are the logical space's, with a third toward the
//! camera: x right, y down (as sprites are placed) and z out of the
//! screen, toward the player. A positive turn about an axis turns the
//! next axis toward the one after it: about x, +y toward +z; about y, +z
//! toward +x; about z, +x toward +y, which is clockwise on screen, the
//! way headings turn.
//!
//! The renderer resolves a [`ModelKey`] to a mesh it was given
//! (`nova_render::Renderer::add_model`) and draws it with a camera that
//! matches the 2D view: a model at (x, y) is centred where a sprite
//! centred at (x, y) is, one model unit spanning [`ModelPose::scale`]
//! logical units.
//!
//! [`DrawCommand::Model`]: crate::DrawCommand::Model
//! [`DrawCommand::Mounted`]: crate::DrawCommand::Mounted

use crate::geometry::Point;

/// Which model a [`DrawCommand::Model`](crate::DrawCommand::Model) draws:
/// a key the renderer was given a mesh for, as an [`ImageKey`] names an
/// image.
///
/// [`ImageKey`]: crate::ImageKey
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelKey(pub u32);

/// How a model is turned: a quaternion, `w + xi + yj + zk`, about the
/// model's axes (see the [module docs](self)).
///
/// It need not be a unit quaternion: the renderer normalises it
/// ([`Orientation::normalized`]), and one that is zero or not finite
/// turns nothing to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orientation {
    /// The real part.
    pub w: f32,
    /// The x part.
    pub x: f32,
    /// The y part.
    pub y: f32,
    /// The z part.
    pub z: f32,
}

impl Orientation {
    /// No turn at all.
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// A turn of `radians` about `axis`, positive as the [module
    /// docs](self) say. The axis need not be a unit vector. A zero or
    /// non-finite axis names no direction, so it turns nothing
    /// ([`Orientation::IDENTITY`]); a non-finite angle gives an
    /// orientation that does not normalise, which draws nothing.
    #[must_use]
    pub fn about(axis: [f32; 3], radians: f32) -> Self {
        let [x, y, z] = axis;
        let length = (x * x + y * y + z * z).sqrt();
        if !length.is_finite() || length == 0.0 {
            return Self::IDENTITY;
        }
        let (sin, cos) = (radians / 2.0).sin_cos();
        let k = sin / length;
        Self {
            w: cos,
            x: x * k,
            y: y * k,
            z: z * k,
        }
    }

    /// This turn, then `next`: a point turned by the result is turned by
    /// `self` first and by `next` after.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        // The Hamilton product `next × self`.
        let (a, b) = (next, self);
        Self {
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        }
    }

    /// This orientation scaled to a unit quaternion, or `None` when it is
    /// zero or not finite.
    #[must_use]
    pub fn normalized(self) -> Option<Self> {
        let Self { w, x, y, z } = self;
        let length = (w * w + x * x + y * y + z * z).sqrt();
        if !length.is_finite() || length == 0.0 {
            return None;
        }
        Some(Self {
            w: w / length,
            x: x / length,
            y: y / length,
            z: z / length,
        })
    }

    /// The rotation it makes, normalised first, as a 3x3 matrix by rows
    /// (a turned point's i-th axis is row i dotted with the point), or
    /// `None` when it does not normalise.
    #[must_use]
    pub fn matrix(self) -> Option<[[f32; 3]; 3]> {
        let Self { w, x, y, z } = self.normalized()?;
        Some([
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ])
    }

    /// `point` turned by it, or `None` when it does not normalise.
    #[must_use]
    pub fn rotate(self, point: [f32; 3]) -> Option<[f32; 3]> {
        let rows = self.matrix()?;
        Some(rows.map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2]))
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Where a model is drawn and how it is turned and sized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelPose {
    /// Where the model's origin goes, in logical units, as a sprite's
    /// centre is placed.
    pub center: Point,
    /// How it is turned.
    pub orientation: Orientation,
    /// Logical units per model unit, along every axis. One that is not a
    /// positive, finite number draws nothing.
    pub scale: f32,
}

/// Where a part mounted on a model sits on it
/// ([`DrawCommand::Mounted`](crate::DrawCommand::Mounted)): at `at` on the
/// model's own axes, turned `turn` about the model's z, so a point `p` of
/// the part is drawn where the model's point `at + R_z(turn) · p` is.
///
/// The part's mesh is in the model's units: a ship's model and its
/// outfits' are both in metres. An `at` or `turn` that isn't finite draws
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mounting {
    /// Where the part's origin sits, on the model's axes in its units.
    pub at: [f32; 3],
    /// How far the part is turned about the model's z axis, in radians,
    /// positive as the [module docs](self) say: clockwise on screen, as
    /// headings turn.
    pub turn: f32,
}

/// The ship-space vector `[starboard, nose, up]` on the view's model axes:
/// starboard is +x (right), the nose −y (up the screen at heading 0) and
/// up +z (toward the camera).
///
/// Ship space is where a ship's model is placed
/// (`nova_galaxy_media::Placement::ship_point_3d`): the renderer maps a
/// loaded model's vertices by it (`nova_render::model_source`), so
/// anything placed on a ship takes this one map. It is a reflection, as
/// these axes are left-handed.
#[must_use]
pub fn view_axes([starboard, nose, up]: [f32; 3]) -> [f32; 3] {
    [starboard, -nose, up]
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    /// Asserts `a` and `b` agree to within a millionth or so on each axis.
    #[track_caller]
    fn assert_near(a: [f32; 3], b: [f32; 3]) {
        for (p, q) in a.iter().zip(&b) {
            assert!((p - q).abs() < 1e-5, "{a:?} is not {b:?}");
        }
    }

    const X: [f32; 3] = [1.0, 0.0, 0.0];
    const Y: [f32; 3] = [0.0, 1.0, 0.0];
    const Z: [f32; 3] = [0.0, 0.0, 1.0];

    fn turned(orientation: Orientation, point: [f32; 3]) -> [f32; 3] {
        orientation.rotate(point).expect("normalises")
    }

    /// `point` turned `radians` about `axis` by Rodrigues' formula,
    /// `v cos θ + (k × v) sin θ + k (k · v)(1 − cos θ)`, k the unit axis:
    /// a reference that knows nothing of quaternions.
    fn rodrigues(axis: [f32; 3], radians: f32, v: [f32; 3]) -> [f32; 3] {
        let length = axis.iter().map(|c| c * c).sum::<f32>().sqrt();
        let k = axis.map(|c| c / length);
        let (sin, cos) = radians.sin_cos();
        let cross = [
            k[1] * v[2] - k[2] * v[1],
            k[2] * v[0] - k[0] * v[2],
            k[0] * v[1] - k[1] * v[0],
        ];
        let along: f32 = (0..3).map(|i| k[i] * v[i]).sum();
        [0, 1, 2].map(|i| v[i] * cos + cross[i] * sin + k[i] * along * (1.0 - cos))
    }

    const POINTS: [[f32; 3]; 4] = [X, Y, Z, [0.4, -1.3, 2.2]];

    #[test]
    #[allow(clippy::float_cmp, reason = "the map moves no bits but a sign")]
    fn ship_space_maps_nose_to_minus_y_and_up_to_plus_z() {
        assert_eq!(view_axes([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        assert_eq!(view_axes([0.0, 2.0, 0.0]), [0.0, -2.0, 0.0]);
        assert_eq!(view_axes([0.0, 0.0, 3.0]), [0.0, 0.0, 3.0]);
        assert_eq!(view_axes([1.0, 2.0, 3.0]), [1.0, -2.0, 3.0]);
    }

    #[test]
    fn a_turn_about_any_axis_turns_points_as_rodrigues_formula_does() {
        for (axis, radians) in [([0.3, -1.0, 0.7], 2.1), ([2.0, 0.5, -1.5], -0.8)] {
            let orientation = Orientation::about(axis, radians);
            for point in POINTS {
                assert_near(turned(orientation, point), rodrigues(axis, radians, point));
            }
        }
    }

    #[test]
    fn then_composes_general_turns_in_order() {
        let first = Orientation::about([0.3, -1.0, 0.7], 2.1);
        let second = Orientation::about([2.0, 0.5, -1.5], -0.8);
        let both = first.then(second);
        for point in POINTS {
            assert_near(turned(both, point), turned(second, turned(first, point)));
        }
    }

    #[test]
    fn normalized_scales_every_part_by_the_quaternions_length() {
        let q = Orientation {
            w: 1.0,
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let unit = q.normalized().expect("finite");
        let length = 30.0_f32.sqrt();
        for (got, want) in [unit.w, unit.x, unit.y, unit.z]
            .iter()
            .zip([1.0, 2.0, 3.0, 4.0].map(|c| c / length))
        {
            assert!((got - want).abs() < 1e-6, "{unit:?}");
        }
    }

    #[test]
    fn orientation_identity_leaves_axes() {
        for axis in [X, Y, Z, [0.3, -2.0, 5.0]] {
            assert_near(turned(Orientation::IDENTITY, axis), axis);
        }
        assert_eq!(Orientation::default(), Orientation::IDENTITY);
    }

    #[test]
    fn a_quarter_turn_about_the_view_axis_turns_x_toward_y() {
        let quarter = Orientation::about(Z, FRAC_PI_2);
        assert_near(turned(quarter, X), Y);
        assert_near(turned(quarter, Y), [-1.0, 0.0, 0.0]);
        assert_near(turned(quarter, Z), Z);
    }

    #[test]
    fn a_quarter_turn_about_x_turns_y_toward_z_and_about_y_z_toward_x() {
        assert_near(turned(Orientation::about(X, FRAC_PI_2), Y), Z);
        assert_near(turned(Orientation::about(Y, FRAC_PI_2), Z), X);
    }

    #[test]
    fn the_axis_need_not_be_a_unit_vector() {
        let long = Orientation::about([0.0, 0.0, 7.0], FRAC_PI_2);
        assert_near(turned(long, X), Y);
        let norm = long.normalized().expect("unit already");
        assert!((norm.w - long.w).abs() < 1e-6 && (norm.z - long.z).abs() < 1e-6);
    }

    #[test]
    fn then_composes_in_order() {
        // About z, then about x: +x goes to +y, then +y to +z.
        let both = Orientation::about(Z, FRAC_PI_2).then(Orientation::about(X, FRAC_PI_2));
        assert_near(turned(both, X), Z);
        // The other order: +x stays put about x, then goes to +y.
        let other = Orientation::about(X, FRAC_PI_2).then(Orientation::about(Z, FRAC_PI_2));
        assert_near(turned(other, X), Y);
    }

    #[test]
    fn about_a_zero_axis_is_rejected_or_identity() {
        assert_eq!(Orientation::about([0.0; 3], 1.0), Orientation::IDENTITY);
        assert_eq!(
            Orientation::about([f32::NAN, 0.0, 1.0], 1.0),
            Orientation::IDENTITY
        );
        assert_eq!(
            Orientation::about([f32::INFINITY, 0.0, 0.0], 1.0),
            Orientation::IDENTITY
        );
    }

    #[test]
    fn a_non_finite_angle_does_not_normalise() {
        assert_eq!(Orientation::about(Z, f32::NAN).normalized(), None);
    }

    #[test]
    fn an_orientation_is_normalised_before_it_turns() {
        let doubled = Orientation {
            w: 2.0,
            ..Orientation::IDENTITY
        };
        assert_near(turned(doubled, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
        let quarter = Orientation::about(Z, FRAC_PI_2);
        let scaled = Orientation {
            w: quarter.w * 3.0,
            x: 0.0,
            y: 0.0,
            z: quarter.z * 3.0,
        };
        assert_near(turned(scaled, X), Y);
    }

    #[test]
    fn a_zero_or_non_finite_orientation_turns_nothing() {
        let zero = Orientation {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let nan = Orientation {
            w: f32::NAN,
            ..Orientation::IDENTITY
        };
        let huge = Orientation {
            w: f32::INFINITY,
            ..Orientation::IDENTITY
        };
        for orientation in [zero, nan, huge] {
            assert_eq!(orientation.normalized(), None, "{orientation:?}");
            assert_eq!(orientation.matrix(), None);
            assert_eq!(orientation.rotate(X), None);
        }
    }
}
