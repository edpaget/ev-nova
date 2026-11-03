//! A particle's place and velocity in the original's units: 1/256 of a
//! pixel, fixed 24.8, stepped by integer addition, as `_NewParticle`
//! stores them (+0x4/+0x8, +0x14/+0x18, @0xc509f-0xc50de) and
//! `_MoveParticles` adds them (@0xc6145-0xc615d).
//!
//! - A particle starts on a whole pixel: `_SpawnParticles` truncates each
//!   piece's start toward zero before shifting it left 8 (@0x44d19-0x44d2a),
//!   [`Fixed::spawned`].
//! - Its velocity is `trunc(v x 256)`, toward zero (@0x44ced-0x44d11),
//!   [`Fixed::velocity`].
//! - It is plotted from its whole pixel, `x >> 8`, an arithmetic shift and
//!   so a floor (`_DrawParticlesOffscreen` @0xc5a29-0xc5a32),
//!   [`Fixed::whole`].
//!
//! The original keeps both in 32-bit ints; these are wider, as the engine
//! does not replicate integer widths.

use crate::Point;

/// How many of these units make a pixel: 8 fractional bits.
pub const UNITS_PER_PIXEL: i64 = 256;

/// A point or a velocity in 1/256 of a pixel (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fixed {
    /// Across, in 1/256 of a pixel.
    pub x: i64,
    /// Down, in 1/256 of a pixel.
    pub y: i64,
}

impl Fixed {
    /// A particle's start at world point `at`: each axis truncated toward
    /// zero to a whole pixel, then shifted left 8 (`_SpawnParticles`
    /// @0x44d19-0x44d2a).
    #[must_use]
    pub fn spawned(at: Point) -> Self {
        // A world coordinate is far within an `i64`.
        #[allow(clippy::cast_possible_truncation)]
        let whole = |v: f32| (v.trunc() as i64) * UNITS_PER_PIXEL;
        Self {
            x: whole(at.x),
            y: whole(at.y),
        }
    }

    /// A velocity of `v` pixels a tick: `trunc(v x 256)` on each axis,
    /// toward zero (`_SpawnParticles` @0x44ced-0x44d11).
    #[must_use]
    pub fn velocity(v: Point) -> Self {
        // `UNITS_PER_PIXEL` is exact in an `f32`, and a speed far within an
        // `i64`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let units = |v: f32| (v * UNITS_PER_PIXEL as f32).trunc() as i64;
        Self {
            x: units(v.x),
            y: units(v.y),
        }
    }

    /// The whole pixel it is on: `x >> 8`, a floor
    /// (`_DrawParticlesOffscreen` @0xc5a29-0xc5a32).
    #[must_use]
    pub fn whole(self) -> (i64, i64) {
        (self.x >> 8, self.y >> 8)
    }

    /// It in pixels, fractional: for readers that want a world point.
    #[must_use]
    pub fn point(self) -> Point {
        // A world coordinate is far within an `f32`'s precision here.
        #[allow(clippy::cast_precision_loss)]
        let pixels = |v: i64| v as f32 / UNITS_PER_PIXEL as f32;
        Point::new(pixels(self.x), pixels(self.y))
    }

    /// It moved by `velocity`, by integer addition (`_MoveParticles`
    /// @0xc6145-0xc615d).
    #[must_use]
    pub fn moved_by(self, velocity: Self) -> Self {
        Self {
            x: self.x + velocity.x,
            y: self.y + velocity.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spawn_is_truncated_toward_zero_to_a_whole_pixel() {
        let at = Fixed::spawned(Point::new(10.75, -3.25));
        assert_eq!(
            at,
            Fixed {
                x: 10 * 256,
                y: -3 * 256
            }
        );
        assert_eq!(at.whole(), (10, -3));
    }

    #[test]
    fn a_velocity_is_truncated_toward_zero_in_256ths() {
        // 0.3 x 256 = 76.8, -0.7 x 256 = -179.2.
        assert_eq!(
            Fixed::velocity(Point::new(0.3, -0.7)),
            Fixed { x: 76, y: -179 }
        );
    }

    #[test]
    fn the_whole_pixel_is_a_floor_so_a_quarter_below_zero_is_minus_one() {
        assert_eq!(Fixed { x: -64, y: 64 }.whole(), (-1, 0));
        assert_eq!(Fixed { x: 511, y: -257 }.whole(), (1, -2));
    }

    #[test]
    fn it_moves_by_integer_addition() {
        let start = Fixed::spawned(Point::new(5.0, 5.0));
        let velocity = Fixed::velocity(Point::new(0.3, -0.7));
        let after = (0..10).fold(start, |at, _| at.moved_by(velocity));
        assert_eq!(
            after,
            Fixed {
                x: 1280 + 760,
                y: 1280 - 1790
            }
        );
        // 2040 / 256 = 7.97, -510 / 256 = -1.99: floors.
        assert_eq!(after.whole(), (7, -2));
    }

    #[test]
    fn its_point_is_its_256ths_in_pixels() {
        assert_eq!(Fixed { x: 640, y: -64 }.point(), Point::new(2.5, -0.25));
    }
}
