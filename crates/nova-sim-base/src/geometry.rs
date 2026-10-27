//! The simulation's own vector: positions and velocities in pixels, x
//! growing right and y growing down, as on screen and in Nova's `spöb`
//! positions.

use std::ops::{Add, Mul, Sub};

/// A 2D vector in pixels (or pixels a tick).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal: grows right.
    pub x: f32,
    /// Vertical: grows down.
    pub y: f32,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// The vector (`x`, `y`).
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Its length.
    #[must_use]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Its squared length, `x·x + y·y` in single precision, as the
    /// original compares distances without taking the root.
    #[must_use]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn vectors_add_subtract_and_scale_per_axis() {
        let (a, b) = (Vec2::new(1.5, -2.0), Vec2::new(0.25, 4.0));
        assert_eq!(a + b, Vec2::new(1.75, 2.0));
        assert_eq!(a - b, Vec2::new(1.25, -6.0));
        assert_eq!(a * 2.0, Vec2::new(3.0, -4.0));
        assert_eq!(Vec2::ZERO, Vec2::default());
        assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn the_length_is_the_hypotenuse() {
        assert_eq!(Vec2::new(3.0, -4.0).length(), 5.0);
        assert_eq!(Vec2::ZERO.length(), 0.0);
        assert_eq!(Vec2::new(0.0, -2.5).length(), 2.5);
    }

    #[test]
    fn the_squared_length_is_x_times_x_plus_y_times_y_in_single_precision() {
        assert_eq!(Vec2::new(3.0, -4.0).length_squared(), 25.0);
        assert_eq!(Vec2::new(300.0, 400.0).length_squared(), 250_000.0);
        assert_eq!(Vec2::ZERO.length_squared(), 0.0);
        let (x, y) = (0.1_f32, 0.7_f32);
        assert_eq!(Vec2::new(x, y).length_squared(), x * x + y * y);
    }
}
