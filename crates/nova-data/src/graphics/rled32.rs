//! The other reading of a 32-bit `rlëD`: true colour at its box's width.
//!
//! The original draws a 32-bit sheet's bytes into its 16-bit world at
//! twice its width ([`super::rled_wide`]), which the Bible does not
//! describe: it calls `rlëD`s 16-bit and says nothing of 32.
//! [`ThirtyTwoBitReading::TrueColour`] reads what such a sheet means
//! instead: each 4-byte pixel is `xRRGGBB`, opaque, a run is one such
//! pixel repeated, and a skip is `count / 4` clear pixels. The rulebook's
//! `thirty_two_bit_sheet` key chooses between them.

use super::image::Image;
use super::rled::Sink;

/// How an `rlëD` whose header says 32 bits is turned into pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThirtyTwoBitReading {
    /// As the original's 16-bit world draws it: each 32-bit pixel is two
    /// little-endian RGB555 pixels, the row twice the box's width from its
    /// left edge ([`super::rled_wide`]).
    #[default]
    AsDrawn,
    /// Each 32-bit pixel is one `xRRGGBB` true-colour pixel, opaque, at the
    /// box's width.
    TrueColour,
}

/// The opaque colour of a 32-bit pixel `x RR GG BB`.
fn true_colour([_, r, g, b]: [u8; 4]) -> [u8; 4] {
    [r, g, b, 255]
}

/// A sink that paints each frame of a 32-bit sheet in true colour, and
/// each frame's runs alone.
pub(super) struct TrueColour {
    width: u32,
    height: u32,
    pub(super) frames: Vec<Image>,
    frame: Image,
    /// Each finished frame's runs, if it had any.
    pub(super) solids: Vec<Option<Image>>,
    /// The current frame's runs, if it has any.
    solid: Option<Image>,
}

impl TrueColour {
    /// An empty sheet of `count` frames, `width` x `height` each.
    pub(super) fn new(width: u32, height: u32, count: u16) -> Self {
        Self {
            width,
            height,
            frames: Vec::with_capacity(usize::from(count)),
            frame: Image::transparent(width, height),
            solids: Vec::with_capacity(usize::from(count)),
            solid: None,
        }
    }
}

impl Sink for TrueColour {
    fn pixels(&mut self, x: u32, y: u32, pixels: &[u8]) {
        for (i, &pixel) in (0..).zip(pixels.as_chunks::<4>().0) {
            self.frame.put(x + i, y, true_colour(pixel));
        }
    }

    fn run(&mut self, x: u32, y: u32, len: u32, word: u32) {
        let colour = true_colour(word.to_be_bytes());
        let (width, height) = (self.width, self.height);
        let solid = self
            .solid
            .get_or_insert_with(|| Image::transparent(width, height));
        for i in 0..len {
            self.frame.put(x + i, y, colour);
            solid.put(x + i, y, colour);
        }
    }

    fn frame_end(&mut self) {
        let next = Image::transparent(self.width, self.height);
        self.frames.push(std::mem::replace(&mut self.frame, next));
        self.solids.push(self.solid.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::GraphicsError;
    use crate::graphics::fixture::RledBuilder;
    use crate::graphics::rled::{RledReadings, decode_rled, decode_rled_with};
    use crate::graphics::rled8::EightBitReading;
    use crate::graphics::sheet::SpriteSheet;

    const CLEAR: [u8; 4] = [0; 4];

    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    fn true_colour_reading() -> RledReadings {
        RledReadings {
            thirty_two_bit: ThirtyTwoBitReading::TrueColour,
            ..RledReadings::default()
        }
    }

    fn through_true_colour(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_with(data, None, true_colour_reading())
    }

    #[test]
    fn the_original_reading_is_the_default() {
        assert_eq!(ThirtyTwoBitReading::default(), ThirtyTwoBitReading::AsDrawn);
        assert_eq!(
            RledReadings::default(),
            RledReadings {
                eight_bit: EightBitReading::AsDrawn,
                thirty_two_bit: ThirtyTwoBitReading::AsDrawn,
            }
        );
    }

    #[test]
    fn through_true_colour_a_32_bit_sheet_is_its_own_width() {
        let bytes = RledBuilder::new(2, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[0x0012_3456, 0x80AB_CDEF]))
            .build();
        let sheet = through_true_colour(&bytes).unwrap();
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (2, 1));
        assert_eq!((sheet.image_width(), sheet.image_height()), (2, 1));
        assert_eq!(sheet.box_origin(), (0, 0));
        assert_eq!(
            rgba(&sheet.frames()[0]),
            [[0x12, 0x34, 0x56, 255], [0xAB, 0xCD, 0xEF, 255]]
        );
        assert_eq!(sheet.solid_frames(), None);
        // The default reading is the original's.
        assert_ne!(decode_rled(&bytes, None).unwrap(), sheet);
    }

    #[test]
    fn through_true_colour_a_run_repeats_one_pixel_and_a_skip_skips_pixels() {
        let bytes = RledBuilder::new(4, 1)
            .depth(32)
            .frame(|f| f.line().skip32(1).run32(2, 0x0011_2233))
            .build();
        let sheet = through_true_colour(&bytes).unwrap();
        let colour = [0x11, 0x22, 0x33, 255];
        let drawn = vec![CLEAR, colour, colour, CLEAR];
        assert_eq!(
            sheet.solid_frames().map(|s| rgba(&s[0])),
            Some(drawn.clone())
        );
        assert_eq!(rgba(&sheet.frames()[0]), drawn);
    }

    #[test]
    fn through_true_colour_pixel_bytes_must_be_whole_pixels() {
        let bytes = RledBuilder::new(2, 1)
            .depth(32)
            .frame(|f| f.line().token(2, 6).raw(&[0; 8]))
            .build();
        assert_eq!(
            through_true_colour(&bytes),
            Err(GraphicsError::OddByteCount { offset: 20 })
        );
        let bytes = RledBuilder::new(2, 1)
            .depth(32)
            .frame(|f| f.line().token(2, 0))
            .build();
        assert_eq!(
            through_true_colour(&bytes),
            Err(GraphicsError::RunawaySwap { offset: 20 })
        );
        // A row holds the box's width of pixels.
        let bytes = RledBuilder::new(1, 1)
            .depth(32)
            .frame(|f| f.line().pixels32(&[1, 2]))
            .build();
        assert_eq!(
            through_true_colour(&bytes),
            Err(GraphicsError::LineOverflow {
                frame: 0,
                line: 0,
                offset: 20
            })
        );
    }

    #[test]
    fn other_depths_decode_the_same_under_either_reading() {
        let sheets = [
            RledBuilder::new(2, 1)
                .frame(|f| f.line().pixels(&[0x7C00]))
                .build(),
            RledBuilder::new(2, 1)
                .depth(8)
                .frame(|f| f.line().pixels8(&[1, 2]))
                .build(),
            RledBuilder::new(2, 1)
                .depth(24)
                .count_lines()
                .frame(|f| f.line().run8(4, [1, 2, 3, 4]))
                .build(),
        ];
        for bytes in sheets {
            assert_eq!(through_true_colour(&bytes), decode_rled(&bytes, None));
        }
    }
}
