//! A sprite's colorize: the original's per-sprite mix of every texel
//! toward one colour, at a level of 32nds, which the murk and an ionized
//! ship's tint write (sprite +0xb0 level, +0xb4 colour).
//!
//! `_BlitPixieRLETranslucent` (@0xc1568) loads the colour, duplicated
//! (@0xc15bc-0xc15cf), and 32 − level (@0xc15d1-0xc15e5);
//! `_BlitPixieTranslucentCopy` (@0xc1140-0xc11ca) mixes each 5-bit
//! channel of the source texel S toward the colour's C, combining the two
//! halves with OR, never add:
//!
//! ```text
//! P = ((S × (32 − level)) >> 5) | ((C × level) >> 5)
//! ```
//!
//! only when 1 ≤ level ≤ 32 (`_overBias` ≤ 31 unsigned, @0xc128d-0xc1297);
//! otherwise P = S. The sprite's factors and translucency then apply to P
//! (@0xc11e4-0xc1275), which the renderer does with the sprite's tint as
//! it does for any sprite. Level 32 gives exactly C.

/// The most a colorize's level can be: at 32 the texel is the colour.
pub const COLORIZE_STEPS: u8 = 32;

/// A sprite's colorize: its level, 1 to 32 to mix at all, and its colour
/// as 5-bit channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Colorize {
    /// The level, in 32nds.
    pub level: u8,
    /// The colour's red, green and blue, each 0 to 31.
    pub rgb5: [u8; 3],
}

impl Colorize {
    /// A colorize at `level` toward `colour`, `00RRGGBB`, its channels
    /// narrowed to 5 bits as the display's RGB555 holds them.
    #[must_use]
    pub fn toward(level: u8, colour: u32) -> Self {
        Self {
            level,
            rgb5: crate::color::rgb555_channels(colour),
        }
    }

    /// A colorize at `level` toward the RGB555 pixel `pixel`.
    #[must_use]
    pub fn toward_pixel(level: u8, pixel: u16) -> Self {
        let channel = |shift: u16| ((pixel >> shift) & 0x1f) as u8;
        Self {
            level,
            rgb5: [channel(10), channel(5), channel(0)],
        }
    }

    /// Whether it mixes at all: a level of 1 to 32.
    #[must_use]
    pub fn mixes(self) -> bool {
        (1..=COLORIZE_STEPS).contains(&self.level)
    }

    /// The 5-bit texel `s5` (red, green, blue) as it mixes it (see the
    /// module docs).
    #[must_use]
    pub fn mix(self, s5: [u8; 3]) -> [u8; 3] {
        if !self.mixes() {
            return s5;
        }
        let level = u16::from(self.level);
        let keep = u16::from(COLORIZE_STEPS) - level;
        std::array::from_fn(|c| {
            let source = (u16::from(s5[c]) * keep) >> 5;
            let colour = (u16::from(self.rgb5[c]) * level) >> 5;
            // At most 31 | 31.
            #[allow(clippy::cast_possible_truncation)]
            let mixed = (source | colour) as u8;
            mixed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_halves_are_ored_not_added() {
        let half = Colorize {
            level: 16,
            rgb5: [31, 0, 21],
        };
        // Red: 31 × 16 >> 5 = 15, | 31 × 16 >> 5 = 15: 15, where adding
        // would give 30.
        assert_eq!(half.mix([31, 31, 10]), [15, 15, 15]);
    }

    #[test]
    fn level_32_is_the_colour_and_no_level_leaves_the_texel() {
        let full = Colorize {
            level: 32,
            rgb5: [3, 17, 30],
        };
        assert_eq!(full.mix([31, 0, 12]), [3, 17, 30]);
        for level in [0, 33, 200] {
            let none = Colorize {
                level,
                rgb5: [3, 17, 30],
            };
            assert!(!none.mixes(), "{level}");
            assert_eq!(none.mix([31, 0, 12]), [31, 0, 12], "{level}");
        }
        assert!(Colorize { level: 1, ..full }.mixes());
    }

    #[test]
    fn each_half_is_floored_on_its_own() {
        let tint = Colorize {
            level: 14,
            rgb5: [5, 0, 31],
        };
        // Source 7 × 18 >> 5 = 3; colour 5 × 14 >> 5 = 2: 3 | 2 = 3.
        // Blue: 0, then 31 × 14 >> 5 = 13.
        assert_eq!(tint.mix([7, 31, 0]), [3, 17, 13]);
    }

    #[test]
    fn a_colour_narrows_to_5_bits_a_channel() {
        assert_eq!(Colorize::toward(14, 0x0002_02ff).rgb5, [0, 0, 31]);
        assert_eq!(Colorize::toward(14, 0x0034_c2ff).rgb5, [6, 24, 31]);
        assert_eq!(Colorize::toward(9, 0x00ff_8008).level, 9);
        assert_eq!(Colorize::toward(9, 0x00ff_8008).rgb5, [31, 16, 1]);
    }

    #[test]
    fn a_pixel_splits_into_its_5_bit_channels() {
        assert_eq!(Colorize::toward_pixel(20, 12).rgb5, [0, 0, 12]);
        assert_eq!(Colorize::toward_pixel(20, 0x7fff).rgb5, [31, 31, 31]);
        assert_eq!(Colorize::toward_pixel(20, 0x7c00 | 0x0060).rgb5, [31, 3, 0]);
    }
}
