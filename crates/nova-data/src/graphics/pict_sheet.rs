//! Cutting a `spïn`'s `PICT` pair into sprite frames, as the original does
//! when the `spïn` has no `rlëD`.
//!
//! `_LoadMasterStellarSprite` (@0x1eb72) and `_LoadSprites` read a `spïn`'s
//! `rlëD` when one exists (@0x1ebee). Otherwise they call
//! `_SWCreateSpriteFromSinglePictXY` (@0x1ece3) with the sprites and mask
//! `PICT`s, the `spïn`'s `xSize` and `ySize`, no frame limit and mask type
//! 1, then `_ConvertSpriteToRLE` (@0x1ed3a). `xTiles` and `yTiles` play no
//! part in the cut; they give only the layout and the count the game uses.
//!
//! - **Drawing the pictures.** Each `PICT` is drawn into a 16-bit world the
//!   size of its `picFrame` (`_SWCreateGWorldFromPict` @0xa7dc7), erased to
//!   white first (`EraseRect` @0xa7ee5): pixels the picture does not cover
//!   are white. QuickDraw's copy into the 16-bit world keeps each
//!   channel's top five bits ([`to_rgb555`]).
//! - **The grid.** With no frame limit there are (picture width / `xSize`)
//!   × (picture height / `ySize`) frames, both integer divisions
//!   (@0xa9ce7-0xa9d25), cut row by row, left to right
//!   (@0xa9f3c-0xaa045). The columns come from the picture, not `xTiles`.
//! - **The mask.** The mask `PICT`'s world is inverted (`_SWInvertGWorld`
//!   @0xa9c9c → `InvertRect` @0xa6ff8), and `_BlitPixieRLEParsePixels`
//!   (@0xc2875) keeps a pixel exactly where that inverted mask is 0
//!   (@0xc2958-0xc297e): where the mask was pure white at RGB555. **White
//!   in the mask shows the sprite; black, grey and every other colour hide
//!   it.** The mask's uncovered pixels were erased to white, so they show.
//!   Shown pixels keep their colour, black included. This is the opposite
//!   polarity to a button cap's mask ([`super::apply_mask`]), so that
//!   function is not used here.
//! - **The same `PICT` for both.** When `MasksID` is `SpritesID`, the mask
//!   comes from the sprite (@0xa9cd0 → @0xa9e30): a pixel whose `& 0x7FFF`
//!   is white (`0x7FFF`, the transparent colour `_gSWTransparentColor`
//!   @0xdf020) is hidden, every other pixel shown (`_SWCreateMaskAndFixImage`
//!   @0xa8671-0xa86b4).
//! - **The whole-grid check.** `_ConvertSpriteToRLE` requires the picture's
//!   area to be the frames' (@0x1e4a0-0x1e4e4): a picture whose width and
//!   height are not multiples of the frame size gets no RLE data.
//!
//! Each of the original's failures here ends the game: a picture smaller
//! than one frame divides by zero (@0xa9f12 → @0xa90e7), a frame size of 0
//! divides by zero (@0xa9cff, @0xa9d1d) and a negative one fails an assert
//! (@0xad835-0xad844), and a picture that is not a whole grid asserts on
//! its first draw (@0xbb43c-0xbb441). A mask narrower or shorter than its
//! sprites is read sheared or past its end, because it is stepped by the
//! sprites' row bytes (@0xc2b88-0xc2b8e). The engine reads each as an
//! error instead, so the caller falls back as it does for a sheet it
//! cannot read.
//!
//! None of these routines plays a sound. The full address record is in
//! `nova_sim_story`'s "Checked for stellar-fidelity phase 4".

use super::GraphicsError;
use super::color::{rgb555, to_rgb555};
use super::image::Image;
use super::sheet::{SheetLayout, SpriteSheet};

/// White at RGB555: every channel's five bits set.
const WHITE_555: u16 = 0x7FFF;

/// Where a `PICT` sprite grid's mask comes from.
#[derive(Clone, Copy, Debug)]
pub enum SheetMask<'a> {
    /// A mask picture of its own: white shows the sprite.
    Picture(&'a Image),
    /// The sprites themselves (`MasksID` is `SpritesID`): white is hidden.
    Itself,
}

/// Cuts `sprites` into `x_size` x `y_size` frames, masked by `mask`, as
/// the original's `_SWCreateSpriteFromSinglePictXY` and
/// `_ConvertSpriteToRLE` do (see the module docs), arranged by `layout`
/// (or [`SheetLayout::DEFAULT`]) when composed.
///
/// A frame size of zero or less is [`GraphicsError::BadDimensions`]; a
/// picture smaller than one frame is [`GraphicsError::NoFrames`]; one that
/// is not a whole grid of frames is [`GraphicsError::PartialGrid`]; a mask
/// of another width, or shorter than the sprites, is
/// [`GraphicsError::MaskSizeMismatch`].
pub fn cut_pict_sheet(
    sprites: &Image,
    mask: SheetMask<'_>,
    x_size: i16,
    y_size: i16,
    layout: Option<SheetLayout>,
) -> Result<SpriteSheet, GraphicsError> {
    let (Ok(frame_width @ 1..), Ok(frame_height @ 1..)) =
        (u32::try_from(x_size), u32::try_from(y_size))
    else {
        return Err(GraphicsError::BadDimensions {
            width: i32::from(x_size),
            height: i32::from(y_size),
        });
    };
    let (width, height) = (sprites.width(), sprites.height());
    let (columns, rows) = (width / frame_width, height / frame_height);
    if columns == 0 || rows == 0 {
        return Err(GraphicsError::NoFrames);
    }
    if width % frame_width != 0 || height % frame_height != 0 {
        return Err(GraphicsError::PartialGrid {
            width,
            height,
            frame_width,
            frame_height,
        });
    }
    if let SheetMask::Picture(mask) = mask
        && (mask.width() != width || mask.height() < height)
    {
        return Err(GraphicsError::MaskSizeMismatch {
            width,
            height,
            mask_width: mask.width(),
            mask_height: mask.height(),
        });
    }
    let shown = |x: u32, y: u32, pixel: u16| match mask {
        SheetMask::Picture(mask) => mask.pixel(x, y).map(to_rgb555) == Some(WHITE_555),
        SheetMask::Itself => pixel & WHITE_555 != WHITE_555,
    };
    let frames = (0..columns * rows)
        .map(|i| {
            let (left, top) = (i % columns * frame_width, i / columns * frame_height);
            let mut frame = Image::transparent(frame_width, frame_height);
            for y in 0..frame_height {
                for x in 0..frame_width {
                    let (sx, sy) = (left + x, top + y);
                    let pixel = sprites.pixel(sx, sy).map_or(WHITE_555, to_rgb555);
                    if shown(sx, sy, pixel) {
                        frame.put(x, y, rgb555(pixel));
                    }
                }
            }
            frame
        })
        .collect();
    let layout = layout.unwrap_or(SheetLayout::DEFAULT);
    Ok(SpriteSheet::new(frame_width, frame_height, frames, layout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::decode_pict;
    use crate::graphics::fixture::{DirectBits, PictBuilder};

    const CLEAR: [u8; 4] = [0; 4];
    const WHITE: [u8; 3] = [0xFF; 3];
    const BLACK: [u8; 3] = [0; 3];

    /// A `width` x `height` picture of these pixels, row by row.
    fn picture(width: i16, height: i16, pixels: &[[u8; 3]]) -> Image {
        let frame = [0, 0, height, width];
        let bytes = PictBuilder::new(frame)
            .direct_bits(&DirectBits::rgb888(frame, pixels))
            .end()
            .build();
        decode_pict(&bytes).expect("decodes")
    }

    /// Pixel `i` of a [`distinct`] picture: (`i`·8, 0x40, 0x80), distinct
    /// at five bits.
    fn nth(i: u32) -> [u8; 3] {
        [(i * 8) as u8, 0x40, 0x80]
    }

    /// A picture whose every pixel is distinct ([`nth`]).
    fn distinct(width: i16, height: i16) -> Image {
        let pixels: Vec<[u8; 3]> = (0..(width * height) as u32).map(nth).collect();
        picture(width, height, &pixels)
    }

    /// An all-white mask.
    fn white(width: i16, height: i16) -> Image {
        picture(width, height, &vec![WHITE; (width * height) as usize])
    }

    /// The colour a picture's `[r, g, b]` draws as in the 16-bit world.
    fn drawn([r, g, b]: [u8; 3]) -> [u8; 4] {
        rgb555((u16::from(r >> 3) << 10) | (u16::from(g >> 3) << 5) | u16::from(b >> 3))
    }

    fn cut(sprites: &Image, mask: SheetMask<'_>, x: i16, y: i16) -> SpriteSheet {
        cut_pict_sheet(sprites, mask, x, y, None).expect("cuts")
    }

    #[test]
    fn the_grid_is_cut_by_the_pictures_size_row_by_row() {
        let sprites = distinct(6, 4);
        let mask = white(6, 4);
        let two = SheetLayout::new(2).unwrap();
        let sheet =
            cut_pict_sheet(&sprites, SheetMask::Picture(&mask), 2, 2, Some(two)).expect("cuts");
        assert_eq!(sheet.frames().len(), 6);
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (2, 2));
        assert_eq!(sheet.layout(), two);
        // Three columns from the picture's width: frame 3 is the cell at
        // (0, 2), and each frame holds its own cell's pixels.
        for (i, frame) in (0..).zip(sheet.frames()) {
            let (left, top) = (i % 3 * 2, i / 3 * 2);
            for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert_eq!(
                    frame.pixel(x, y),
                    Some(drawn(nth((top + y) * 6 + left + x))),
                    "frame {i} at ({x}, {y})"
                );
            }
        }
        assert_eq!(sheet.frames()[3].pixel(0, 0), Some(drawn(nth(12))));
        // The layout changes only how the frames are composed.
        let default = cut(&sprites, SheetMask::Picture(&mask), 2, 2);
        assert_eq!(default.layout(), SheetLayout::DEFAULT);
        assert_eq!(default.frames(), sheet.frames());
    }

    #[test]
    fn colours_keep_their_top_five_bits() {
        let sprites = picture(2, 1, &[[0x12, 0x34, 0x56], [0xFF, 0x07, 0xF8]]);
        let sheet = cut(&sprites, SheetMask::Picture(&white(2, 1)), 2, 1);
        assert_eq!(
            sheet.frames()[0].pixel(0, 0),
            Some(rgb555(
                ((0x12 >> 3) << 10) | ((0x34 >> 3) << 5) | (0x56 >> 3)
            ))
        );
        assert_eq!(sheet.frames()[0].pixel(1, 0), Some(rgb555(0x7C1F)));
    }

    #[test]
    fn white_in_the_mask_shows_the_sprite_and_every_other_colour_hides_it() {
        let sprites = picture(
            6,
            1,
            &[[0x80, 0x40, 0x20], BLACK, [9; 3], [9; 3], [9; 3], [9; 3]],
        );
        let mask = picture(
            6,
            1,
            &[
                WHITE,
                [0xF8, 0xF8, 0xF8],
                [0xF7, 0xFF, 0xFF],
                [0xFF, 0xFF, 0xF0],
                [0x80, 0x80, 0x80],
                BLACK,
            ],
        );
        let sheet = cut(&sprites, SheetMask::Picture(&mask), 6, 1);
        let shown = |c| Some(drawn(c));
        let frame = &sheet.frames()[0];
        assert_eq!(frame.pixel(0, 0), shown([0x80, 0x40, 0x20]));
        // White at five bits shows a black sprite pixel, opaque.
        assert_eq!(frame.pixel(1, 0), Some([0, 0, 0, 255]));
        for x in 2..6 {
            assert_eq!(frame.pixel(x, 0), Some(CLEAR), "{x}");
        }
    }

    /// Pictures whose pixel opcode covers only their left pixel: the right
    /// one is erased white.
    fn left_only(pixel: [u8; 3]) -> Image {
        let bytes = PictBuilder::new([0, 0, 1, 2])
            .direct_bits(&DirectBits::rgb888([0, 0, 1, 1], &[pixel]))
            .end()
            .build();
        decode_pict(&bytes).expect("decodes")
    }

    #[test]
    fn pixels_neither_picture_covers_are_white() {
        // An uncovered sprite pixel draws white.
        let sheet = cut(&left_only(BLACK), SheetMask::Picture(&white(2, 1)), 2, 1);
        assert_eq!(sheet.frames()[0].pixel(1, 0), Some([255; 4]));
        // An uncovered mask pixel shows; its black neighbour hides.
        let sheet = cut(&distinct(2, 1), SheetMask::Picture(&left_only(BLACK)), 2, 1);
        assert_eq!(sheet.frames()[0].pixel(0, 0), Some(CLEAR));
        assert_eq!(sheet.frames()[0].pixel(1, 0), Some(drawn([8, 0x40, 0x80])));
    }

    #[test]
    fn a_sprite_that_is_its_own_mask_hides_its_white() {
        let sprites = picture(
            5,
            1,
            &[
                WHITE,
                [0xF8, 0xF9, 0xFA],
                [0xF7, 0xFF, 0xFF],
                BLACK,
                [0x80, 0x40, 0x20],
            ],
        );
        let sheet = cut(&sprites, SheetMask::Itself, 5, 1);
        let frame = &sheet.frames()[0];
        assert_eq!(frame.pixel(0, 0), Some(CLEAR));
        assert_eq!(frame.pixel(1, 0), Some(CLEAR));
        assert_eq!(frame.pixel(2, 0), Some(drawn([0xF7, 0xFF, 0xFF])));
        assert_eq!(frame.pixel(3, 0), Some([0, 0, 0, 255]));
        assert_eq!(frame.pixel(4, 0), Some(drawn([0x80, 0x40, 0x20])));
        // An uncovered pixel is white, so it is hidden.
        let sheet = cut(&left_only(BLACK), SheetMask::Itself, 2, 1);
        assert_eq!(sheet.frames()[0].pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(sheet.frames()[0].pixel(1, 0), Some(CLEAR));
    }

    #[test]
    fn a_picture_that_is_not_a_whole_grid_is_an_error() {
        let partial =
            |width, height| cut_pict_sheet(&distinct(width, height), SheetMask::Itself, 2, 2, None);
        assert_eq!(
            partial(5, 4),
            Err(GraphicsError::PartialGrid {
                width: 5,
                height: 4,
                frame_width: 2,
                frame_height: 2,
            })
        );
        assert_eq!(
            partial(4, 3),
            Err(GraphicsError::PartialGrid {
                width: 4,
                height: 3,
                frame_width: 2,
                frame_height: 2,
            })
        );
        assert!(partial(4, 4).is_ok());
    }

    #[test]
    fn a_frame_size_of_zero_or_less_is_bad_dimensions() {
        let sprites = distinct(2, 2);
        for (x, y) in [(0, 1), (1, 0), (-1, 1), (1, -2)] {
            assert_eq!(
                cut_pict_sheet(&sprites, SheetMask::Itself, x, y, None),
                Err(GraphicsError::BadDimensions {
                    width: i32::from(x),
                    height: i32::from(y),
                }),
                "{x}x{y}"
            );
        }
    }

    #[test]
    fn a_picture_smaller_than_a_frame_has_no_frames() {
        let sprites = distinct(2, 2);
        for (x, y) in [(3, 1), (1, 3)] {
            assert_eq!(
                cut_pict_sheet(&sprites, SheetMask::Itself, x, y, None),
                Err(GraphicsError::NoFrames),
                "{x}x{y}"
            );
        }
    }

    #[test]
    fn a_mask_of_another_width_or_shorter_is_a_size_mismatch() {
        let sprites = distinct(2, 2);
        for (width, height) in [(3, 2), (1, 2), (2, 1)] {
            let mask = white(width, height);
            assert_eq!(
                cut_pict_sheet(&sprites, SheetMask::Picture(&mask), 1, 1, None),
                Err(GraphicsError::MaskSizeMismatch {
                    width: 2,
                    height: 2,
                    mask_width: width as u32,
                    mask_height: height as u32,
                }),
                "{width}x{height}"
            );
        }
        // A taller mask is read at the same coordinates.
        let mut pixels = vec![WHITE, BLACK, BLACK, WHITE];
        pixels.extend([BLACK; 2]);
        let mask = picture(2, 3, &pixels);
        let sheet = cut(&sprites, SheetMask::Picture(&mask), 2, 2);
        let frame = &sheet.frames()[0];
        assert_eq!(frame.pixel(0, 0), Some(drawn([0, 0x40, 0x80])));
        assert_eq!(frame.pixel(1, 0), Some(CLEAR));
        assert_eq!(frame.pixel(0, 1), Some(CLEAR));
        assert_eq!(frame.pixel(1, 1), Some(drawn([24, 0x40, 0x80])));
    }
}
