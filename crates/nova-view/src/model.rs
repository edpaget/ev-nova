//! 3D models in the draw list: which model a [`DrawCommand::Model`]
//! draws ([`ModelKey`]) and where and how it is turned ([`ModelPose`],
//! [`Orientation`]).
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
