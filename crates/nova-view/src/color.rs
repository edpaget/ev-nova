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
    /// Error messages' colour, shared by every screen: an opaque light red.
    pub const ERROR: Self = Self::rgba(255, 96, 96, 255);
    /// Secondary text's colour (footers, help and hint lines), shared by
    /// every screen and the app's router: an opaque mid grey.
    pub const DIM: Self = Self::rgba(160, 160, 160, 255);

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

    /// The opaque colour of 5-bit channels `rgb5` (red, green, blue, each
    /// 0 to 31), widened to 8 bits by bit replication,
    /// `(c5 << 3) | (c5 >> 2)`, as the renderer widens an RGB555 texel.
    #[must_use]
    pub const fn from_rgb555(rgb5: [u8; 3]) -> Self {
        let [r, g, b] = rgb5;
        Self::rgba(widen5(r), widen5(g), widen5(b), 255)
    }

    /// The opaque colour a 24-bit `00RRGGBB` value shows as on the
    /// original's thousands-of-colours (RGB555) display: each channel
    /// narrowed to its top 5 bits ([`rgb555_channels`]) and widened back
    /// ([`Color::from_rgb555`]).
    #[must_use]
    pub const fn from_rgb24_as_rgb555(raw: u32) -> Self {
        Self::from_rgb555(rgb555_channels(raw))
    }
}

/// A 5-bit channel widened to 8 bits by bit replication.
const fn widen5(c5: u8) -> u8 {
    (c5 << 3) | (c5 >> 2)
}

/// The red, green and blue of a 24-bit `00RRGGBB` value as the display's
/// RGB555 holds them: each byte's top 5 bits, `byte >> 3`.
#[must_use]
pub const fn rgb555_channels(raw: u32) -> [u8; 3] {
    let [_, r, g, b] = raw.to_be_bytes();
    [r >> 3, g >> 3, b >> 3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_24_bit_colour_shows_at_rgb555_each_channel_narrowed_then_widened() {
        let channel = |byte: u32| Color::from_rgb24_as_rgb555(byte << 16).r;
        assert_eq!(channel(0x07), 0x00, "below a 5-bit step");
        assert_eq!(channel(0x0F), 0x08);
        assert_eq!(channel(0xFF), 0xFF);
        assert_eq!(
            Color::from_rgb24_as_rgb555(0xAB12_3456),
            Color::rgba(0x10, 0x31, 0x52, 255),
            "the top byte ignored"
        );
        assert_eq!(rgb555_channels(0x0012_3456), [2, 6, 10]);
        assert_eq!(
            Color::from_rgb555([31, 0, 16]),
            Color::rgba(255, 0, 132, 255)
        );
    }

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
    fn the_shared_text_colours_are_opaque_red_and_grey() {
        assert_eq!(Color::ERROR, Color::rgba(255, 96, 96, 255));
        assert_eq!(Color::DIM, Color::rgba(160, 160, 160, 255));
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
