//! Which image the resource browser previews for a resource.
//!
//! `PICT`s preview as their one picture and `rlëD`s as their frames, one
//! at a time: a whole sheet composed can be far taller than a GPU texture
//! (the largest stock sheet would be 15360 pixels high). Any other type has
//! no preview yet.

use nova_data::graphics::{Image, PICT, RLED, decode_pict, decode_rled};
use nova_rsrc::ResType;

/// The longest side, in pixels, of a frame the browser previews: half of
/// wgpu's default 8192 texture limit, so a plug-in picture that is very
/// long on one side still fits with room to spare.
pub const MAX_PREVIEW_SIDE: u32 = 4096;

/// What the browser shows for a resource's image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    /// The type has no image preview.
    None,
    /// The frames, previewed one at a time: one for a `PICT`, every frame
    /// of an `rlëD`. Never empty.
    Frames(Vec<Image>),
    /// A frame is wider or taller than [`MAX_PREVIEW_SIDE`].
    TooLarge {
        /// The frame's width in pixels.
        width: u32,
        /// The frame's height in pixels.
        height: u32,
    },
    /// The image did not decode: the error's text.
    Failed(String),
}

/// The preview of resource bytes `data` of type `ty`.
#[must_use]
pub fn preview(ty: ResType, data: &[u8]) -> Preview {
    let frames = if ty == PICT {
        decode_pict(data).map(|image| vec![image])
    } else if ty == RLED {
        decode_rled(data, None).map(nova_data::graphics::SpriteSheet::into_frames)
    } else {
        return Preview::None;
    };
    match frames {
        Ok(frames) => match frames.first() {
            Some(frame)
                if frame.width() > MAX_PREVIEW_SIDE || frame.height() > MAX_PREVIEW_SIDE =>
            {
                Preview::TooLarge {
                    width: frame.width(),
                    height: frame.height(),
                }
            }
            _ => Preview::Frames(frames),
        },
        Err(error) => Preview::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::graphics::{RLED, decode_pict, decode_rled};

    use super::*;

    /// A `width` × `height` RGB555 picture whose pixels count up from 0.
    fn picture(width: i16, height: i16) -> Vec<u8> {
        let bounds = [0, 0, height, width];
        let pixels: Vec<u16> = (0..i32::from(width) * i32::from(height))
            .map(|i| (i % 0x7FFF) as u16)
            .collect();
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &pixels))
            .end()
            .build()
    }

    /// A sheet of `frames` frames of `width` × `height`, frame `n` all
    /// colour `n`.
    fn sheet(frames: u16, width: u16, height: u16) -> Vec<u8> {
        (0..frames)
            .fold(RledBuilder::new(width, height), |sheet, n| {
                sheet.frame(|f| {
                    (0..height).fold(f, |f, _| f.line().pixels(&vec![0x0400 * n; width.into()]))
                })
            })
            .build()
    }

    #[test]
    fn a_picture_previews_as_its_one_frame() {
        let bytes = picture(4, 2);
        let expected = decode_pict(&bytes).expect("decodes");
        assert_eq!((expected.width(), expected.height()), (4, 2));
        assert_eq!(preview(PICT, &bytes), Preview::Frames(vec![expected]));
    }

    #[test]
    fn a_sprite_sheet_previews_as_its_frames() {
        let bytes = sheet(3, 6, 5);
        let Preview::Frames(frames) = preview(RLED, &bytes) else {
            panic!("no frames");
        };
        assert_eq!(frames.len(), 3);
        assert!(frames.iter().all(|f| (f.width(), f.height()) == (6, 5)));
        let expected = decode_rled(&bytes, None).expect("decodes").into_frames();
        assert_eq!(frames, expected);
        assert_ne!(frames[0], frames[1], "the frames differ");
    }

    #[test]
    fn garbage_fails_with_the_decoders_error() {
        let garbage = [0x12, 0x34, 0x56];
        let pict_error = decode_pict(&garbage).expect_err("garbage").to_string();
        let rled_error = decode_rled(&garbage, None)
            .expect_err("garbage")
            .to_string();
        assert_eq!(preview(PICT, &garbage), Preview::Failed(pict_error));
        assert_eq!(preview(RLED, &garbage), Preview::Failed(rled_error));
    }

    #[test]
    fn other_types_have_no_preview() {
        let bytes = picture(4, 2);
        for code in [*b"cicn", *b"snd ", [b's', b'h', 0x95, b'p']] {
            assert_eq!(preview(ResType::new(code), &bytes), Preview::None);
        }
    }

    #[test]
    fn a_frame_over_the_largest_side_is_too_large() {
        let max = MAX_PREVIEW_SIDE as i16;
        assert_eq!(MAX_PREVIEW_SIDE, 4096);
        assert_eq!(
            preview(PICT, &picture(max + 1, 1)),
            Preview::TooLarge {
                width: MAX_PREVIEW_SIDE + 1,
                height: 1
            }
        );
        assert_eq!(
            preview(PICT, &picture(1, max + 1)),
            Preview::TooLarge {
                width: 1,
                height: MAX_PREVIEW_SIDE + 1
            }
        );
        assert!(matches!(
            preview(PICT, &picture(max, 1)),
            Preview::Frames(frames) if frames[0].width() == MAX_PREVIEW_SIDE
        ));
        assert!(matches!(
            preview(PICT, &picture(1, max)),
            Preview::Frames(frames) if frames[0].height() == MAX_PREVIEW_SIDE
        ));
    }

    #[test]
    fn a_sheet_with_frames_over_the_largest_side_is_too_large() {
        let side = MAX_PREVIEW_SIDE as u16 + 1;
        assert_eq!(
            preview(RLED, &sheet(1, side, 1)),
            Preview::TooLarge {
                width: u32::from(side),
                height: 1
            }
        );
    }
}
