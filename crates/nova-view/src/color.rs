//! Colours.

/// An 8-bit-per-channel colour with straight (not premultiplied) alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha: 0 is transparent, 255 opaque.
    pub a: u8,
}

impl Color {
    /// Opaque white.
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    /// Opaque black.
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);

    /// The colour (`r`, `g`, `b`) with alpha `a`.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_opaque_white_and_black() {
        assert_eq!(
            Color::WHITE,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        assert_eq!(
            Color::BLACK,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
    }

    #[test]
    fn rgba_sets_each_channel() {
        assert_eq!(
            Color::rgba(1, 2, 3, 4),
            Color {
                r: 1,
                g: 2,
                b: 3,
                a: 4
            }
        );
    }
}
