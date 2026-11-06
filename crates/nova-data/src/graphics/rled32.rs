//! The other reading of a 32-bit `rlëD`: true colour at its box's width.
//!
//! The original draws a 32-bit sheet's bytes into its 16-bit world at
//! twice its width ([`super::rled_wide`]), which the Bible does not
//! describe: it calls `rlëD`s 16-bit and says nothing of 32.
//! [`ThirtyTwoBitReading::TrueColour`] reads what such a sheet means
//! instead: each 4-byte pixel is `xRRGGBB`, opaque, a run is one such
//! pixel repeated, and a skip is `count / 4` clear pixels. The rulebook's
//! `thirty_two_bit_sheet` key chooses between them.
//!
//! The reading changes only the colours. Whether a sheet loads, and the
//! size of its box and images, stay the original's under both readings,
//! because the store decides graphic type 0's fallback, the frame size
//! and the image size by the original's reading, and touches by its
//! tokens. So a true-colour sheet is the original's pipeline (the swap,
//! the lock walk and the byte draw, with every error they raise) read
//! back four bytes to a pixel: the drawn row's bytes `4k..4k + 4` are
//! pixel `k` of the box. A pixel with a byte left unwritten is clear,
//! and what is drawn past the box's right or bottom edge is dropped, so
//! the overhang the original draws stays clear about the box.

/// How an `rlëD` whose header says 32 bits is turned into pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThirtyTwoBitReading {
    /// As the original's 16-bit world draws it: each 32-bit pixel is two
    /// little-endian RGB555 pixels, the row twice the box's width from its
    /// left edge ([`super::rled_wide`]).
    #[default]
    AsDrawn,
    /// Each 32-bit pixel is one `xRRGGBB` true-colour pixel, opaque, at the
    /// box's width, in images of the drawn sheet's size (see the module
    /// docs).
    TrueColour,
}

/// The opaque colour of the 32-bit pixel whose four bytes lie in the
/// swapped memory as `memory`: the file's `x RR GG BB`, which the swap
/// reverses to `BB GG RR x` (@0xbe46e-0xbe48e), as a run word's
/// `_BlitPixieMemSet` lays it down too.
pub(super) fn true_colour([b, g, r, _]: [u8; 4]) -> [u8; 4] {
    [r, g, b, 255]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::GraphicsError;
    use crate::graphics::fixture::RledBuilder;
    use crate::graphics::image::Image;
    use crate::graphics::rled::{RledReadings, decode_rled, decode_rled_with};
    use crate::graphics::rled8::EightBitReading;
    use crate::graphics::sheet::SpriteSheet;

    const CLEAR: [u8; 4] = [0; 4];

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

    /// What decides whether a sheet is drawn and where: its box, its
    /// images' size, the box's place in them, its frame count, and whether
    /// it has solid frames.
    type Geometry = ((u32, u32), (u32, u32), (u32, u32), usize, bool);

    fn geometry(sheet: &SpriteSheet) -> Geometry {
        (
            (sheet.frame_width(), sheet.frame_height()),
            (sheet.image_width(), sheet.image_height()),
            sheet.box_origin(),
            sheet.frames().len(),
            sheet.solid_frames().is_some(),
        )
    }

    /// The pixels of `image`, one of `sheet`'s, inside the sheet's box,
    /// row by row. Every pixel outside the box must be clear.
    fn in_box(sheet: &SpriteSheet, image: &Image) -> Vec<[u8; 4]> {
        let (left, top) = sheet.box_origin();
        let (right, bottom) = (left + sheet.frame_width(), top + sheet.frame_height());
        let mut inside = Vec::new();
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.pixel(x, y).unwrap();
                if (left..right).contains(&x) && (top..bottom).contains(&y) {
                    inside.push(pixel);
                } else {
                    assert_eq!(pixel, CLEAR, "({x}, {y}) is outside the box");
                }
            }
        }
        inside
    }

    #[test]
    fn through_true_colour_a_32_bit_sheet_colours_its_box() {
        let bytes = RledBuilder::new(2, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[0x0012_3456, 0x80AB_CDEF]))
            .build();
        let sheet = through_true_colour(&bytes).unwrap();
        let drawn = decode_rled(&bytes, None).unwrap();
        // Its images are the drawn sheet's, box and padding alike.
        assert_eq!(geometry(&sheet), geometry(&drawn));
        assert_eq!((sheet.image_width(), sheet.box_origin()), (6, (2, 0)));
        assert_eq!(
            in_box(&sheet, &sheet.frames()[0]),
            [[0x12, 0x34, 0x56, 255], [0xAB, 0xCD, 0xEF, 255]]
        );
        assert_eq!(sheet.solid_frames(), None);
        // The default reading is the original's.
        assert_ne!(drawn, sheet);
    }

    #[test]
    fn through_true_colour_a_run_repeats_one_pixel_and_a_skip_skips_pixels() {
        let bytes = RledBuilder::new(4, 1)
            .depth(32)
            .count_lines()
            .frame(|f| {
                f.line()
                    .skip32(1)
                    .run32(2, 0x0011_2233)
                    .pixels32(&[0x0044_5566])
            })
            .build();
        let sheet = through_true_colour(&bytes).unwrap();
        let (colour, pixel) = ([0x11, 0x22, 0x33, 255], [0x44, 0x55, 0x66, 255]);
        let solid = sheet.solid_frames().expect("solid frames");
        // The solid frame holds the runs alone.
        assert_eq!(in_box(&sheet, &solid[0]), [CLEAR, colour, colour, CLEAR]);
        assert_eq!(
            in_box(&sheet, &sheet.frames()[0]),
            [CLEAR, colour, colour, pixel]
        );
    }

    /// The reading changes only the colours: a sheet loads, or fails, as
    /// the original's does, with the same images about the same box, so
    /// the store, which decides the fallback and the sizes by the
    /// original's reading, agrees with what is drawn.
    #[test]
    fn through_true_colour_a_sheet_loads_or_fails_as_the_drawn_one_does() {
        let sheets = [
            (
                "in sync",
                RledBuilder::new(2, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().pixels32(&[1, 2])),
                true,
            ),
            (
                "a row wider than its box",
                RledBuilder::new(1, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().pixels32(&[1, 2])),
                true,
            ),
            (
                "more line starts than rows",
                RledBuilder::new(1, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().pixels32(&[1]).line().pixels32(&[2])),
                true,
            ),
            (
                "a pixel tail",
                RledBuilder::new(2, 1).depth(32).count_lines().frame(|f| {
                    f.line()
                        .token(2, 6)
                        .raw(&[0x00, 0x11, 0x22, 0x33, 0x03, 0x44, 0x00, 0x00])
                }),
                true,
            ),
            (
                "line starts without their byte counts",
                RledBuilder::new(2, 1)
                    .depth(32)
                    .frame(|f| f.line().pixels32(&[1, 2])),
                false,
            ),
            (
                "a pixel token of no bytes",
                RledBuilder::new(2, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().token(2, 0)),
                false,
            ),
        ];
        for (why, sheet, loads) in sheets {
            let bytes = sheet.build();
            let drawn = decode_rled(&bytes, None);
            assert_eq!(drawn.is_ok(), loads, "{why}");
            assert_eq!(
                through_true_colour(&bytes).map(|sheet| geometry(&sheet)),
                drawn.map(|sheet| geometry(&sheet)),
                "{why}"
            );
        }
    }

    #[test]
    fn through_true_colour_what_is_drawn_outside_the_box_is_dropped() {
        let one = [0, 0, 1, 255];
        let wide = RledBuilder::new(1, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[1, 2]))
            .build();
        let sheet = through_true_colour(&wide).unwrap();
        assert_eq!(in_box(&sheet, &sheet.frames()[0]), [one]);
        let deep = RledBuilder::new(1, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[1]).line().pixels32(&[2]))
            .build();
        let sheet = through_true_colour(&deep).unwrap();
        assert_eq!(in_box(&sheet, &sheet.frames()[0]), [one]);
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
