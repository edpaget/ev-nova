//! The Mac's standard 256-colour table (`clut` 8), computed rather than
//! checked in.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4, "Color
//! QuickDraw", the default colour table for 8-bit devices: index 0 is
//! white, indexes 0 to 214 run through a 6x6x6 cube of the levels `FF`,
//! `CC`, `99`, `66`, `33` and `00` (red slowest, blue fastest) without the
//! cube's black, then four ramps of the ten levels the cube lacks (`EE`,
//! `DD`, `BB`, `AA`, `88`, `77`, `55`, `44`, `22`, `11`): red (215 to 224),
//! green (225 to 234), blue (235 to 244) and grey (245 to 254). Index 255 is
//! black.

/// The cube's levels, brightest first.
const CUBE: [u8; 6] = [0xFF, 0xCC, 0x99, 0x66, 0x33, 0x00];

/// The ramps' levels, brightest first: the 16 multiples of `0x11` that the
/// cube does not hold, without white and black.
const RAMP: [u8; 10] = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11];

/// Indexes the cube covers (its black, index 215 of a full cube, is left
/// out).
const CUBE_LEN: u8 = 215;

/// The opaque RGBA8 colour of `index` in the standard 256-colour table.
pub(crate) fn system_colour(index: u8) -> [u8; 4] {
    if index < CUBE_LEN {
        let i = usize::from(index);
        return [CUBE[i / 36], CUBE[i / 6 % 6], CUBE[i % 6], 255];
    }
    // 215 to 254 are four ramps of ten; 255 is black.
    let ramp = usize::from(index - CUBE_LEN);
    let level = RAMP[ramp % 10];
    match ramp / 10 {
        0 => [level, 0, 0, 255],
        1 => [0, level, 0, 255],
        2 => [0, 0, level, 255],
        3 => [level, level, level, 255],
        _ => [0, 0, 0, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(index: u8) -> u32 {
        let [r, g, b, a] = system_colour(index);
        assert_eq!(a, 255, "{index} is opaque");
        u32::from_be_bytes([0, r, g, b])
    }

    #[test]
    fn the_standard_table_is_the_6_cube_then_four_ramps_then_black() {
        assert_eq!(rgb(0), 0xFF_FFFF);
        assert_eq!(rgb(1), 0xFF_FFCC);
        assert_eq!(rgb(5), 0xFF_FF00);
        assert_eq!(rgb(6), 0xFF_CCFF);
        assert_eq!(rgb(35), 0xFF_0000);
        assert_eq!(rgb(36), 0xCC_FFFF);
        assert_eq!(rgb(214), 0x00_0033);
        let ramp = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11];
        for (i, level) in (0..).zip(ramp) {
            assert_eq!(rgb(215 + i), level << 16, "red {i}");
            assert_eq!(rgb(225 + i), level << 8, "green {i}");
            assert_eq!(rgb(235 + i), level, "blue {i}");
            assert_eq!(rgb(245 + i), level * 0x01_0101, "grey {i}");
        }
        assert_eq!(rgb(255), 0x00_0000);
    }

    #[test]
    fn every_colour_is_distinct() {
        let mut colours: Vec<u32> = (0..=255).map(rgb).collect();
        colours.sort_unstable();
        colours.dedup();
        assert_eq!(colours.len(), 256);
    }
}
