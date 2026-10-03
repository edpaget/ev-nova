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

    /// The opaque colour of a 24-bit `00RRGGBB` value, as Nova stores
    /// colours (a `gövt`'s map colour, for one). The top byte is ignored.
    #[must_use]
    pub const fn from_rgb24(raw: u32) -> Self {
        let [_, r, g, b] = raw.to_be_bytes();
        Self::rgba(r, g, b, 255)
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
    fn a_24_bit_value_is_an_opaque_colour_ignoring_the_top_byte() {
        assert_eq!(
            Color::from_rgb24(0x0012_3456),
            Color::rgba(0x12, 0x34, 0x56, 255)
        );
        assert_eq!(
            Color::from_rgb24(0xAB2C_2CAF),
            Color::rgba(0x2C, 0x2C, 0xAF, 255)
        );
        assert_eq!(Color::from_rgb24(0), Color::BLACK);
        assert_eq!(Color::from_rgb24(0x00FF_FFFF), Color::WHITE);
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
