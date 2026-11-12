//! An image read as pixels: a PNG or a JPEG, decoded to 8-bit RGBA.
//!
//! [`read_image`] reads one of a model's images (the renderer's model
//! adapter, `nova_render::GalaxyModels`, hands it each image a material's
//! base colour names). It checks what the image may cost before it
//! decodes it, in [`read_texture`](crate::texture::read_texture)'s order:
//!
//! 1. which format it is, sniffed from its first bytes ([`format`]), not
//!    taken from a MIME type or a file name: a `.gltf` image named by URI
//!    may give neither;
//! 2. its size, from its header alone;
//! 3. its longest side against the budget's `side` (the default budget's
//!    `texture`, [`budget::texture_limits`](crate::budget::texture_limits));
//! 4. its longest side against the most the caller can draw (its GPU's
//!    largest texture).
//!
//! Only then is it decoded. The budget's `kib` is not checked here: a
//! model's own `kib` already counts its images' bytes
//! ([`Scene::disk_bytes`](crate::scene::Scene::disk_bytes)).
//!
//! A PNG of any colour type and bit depth decodes to RGBA8, as a texture
//! asset's does ([`texture`](crate::texture) decodes through this
//! module). A JPEG decodes to RGBA8 with every alpha 255, strictly: one
//! that ends early or holds corrupt data is refused, not drawn in part.

use std::io::Cursor;

use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::budget::{Cost, Limits, Overrun, over_budget};

/// An image format the engine reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
}

/// An image's pixels, as decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// Its pixels, 8-bit RGBA, row by row from the top: four bytes a
    /// pixel.
    pub rgba: Vec<u8>,
}

/// Why an image has no pixels to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoImage {
    /// It is neither a PNG nor a JPEG, by its first bytes.
    Unknown,
    /// Its longest side is over its art budget's `side`, found from its
    /// header.
    OverBudget(Vec<Overrun>),
    /// Its longest side is over the most the caller can draw (the GPU's
    /// largest texture), found from its header.
    TooLarge {
        /// Its longest side in pixels.
        side: u32,
        /// The most the caller can draw.
        most: u32,
    },
    /// It starts as a PNG or a JPEG does but doesn't decode: its header
    /// gives no size, or its data is cut short or corrupt. Why.
    Undecodable(String),
}

/// The PNG signature.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// How a JPEG starts: the start-of-image marker, then a marker's first
/// byte.
const JPEG_START: &[u8] = &[0xFF, 0xD8, 0xFF];

/// Which format `bytes` holds, by how it starts; `None` for anything the
/// engine doesn't read.
#[must_use]
pub fn format(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(PNG_SIGNATURE) {
        Some(Format::Png)
    } else if bytes.starts_with(JPEG_START) {
        Some(Format::Jpeg)
    } else {
        None
    }
}

/// The image `bytes` holds, held to `limits`' `side` and to `max_side`,
/// the longest side the caller can draw. The format, the budget and the
/// side are checked from the first bytes and the header, before the image
/// is decoded (see the [module docs](self)).
pub fn read_image(bytes: &[u8], limits: &Limits, max_side: u32) -> Result<Pixels, NoImage> {
    let format = format(bytes).ok_or(NoImage::Unknown)?;
    let header = match format {
        Format::Png => png_size(bytes),
        Format::Jpeg => jpeg_size(bytes),
    };
    let (width, height) =
        header.ok_or_else(|| NoImage::Undecodable("its header gives no size".to_owned()))?;
    let side = width.max(height);
    // No bytes: the model's `kib` counts them, so only `side` can be over.
    let over = over_budget(limits, Cost::texture(0, side));
    if !over.is_empty() {
        return Err(NoImage::OverBudget(over));
    }
    if side > max_side {
        return Err(NoImage::TooLarge {
            side,
            most: max_side,
        });
    }
    let rgba = match format {
        Format::Png => decode_png(bytes).map_err(|e| e.to_string()),
        Format::Jpeg => decode_jpeg(bytes).map_err(|e| e.to_string()),
    }
    .map_err(NoImage::Undecodable)?;
    Ok(Pixels {
        width,
        height,
        rgba,
    })
}

/// The width and height a PNG's header gives, read from its first chunk
/// (`IHDR`) alone; `None` when `bytes` doesn't start as a PNG does.
pub(crate) fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if !bytes.starts_with(PNG_SIGNATURE) || bytes.get(12..16) != Some(b"IHDR") {
        return None;
    }
    Some((be(16)?, be(20)?))
}

/// `bytes`, a whole PNG of any colour type and depth, as 8-bit RGBA.
pub(crate) fn decode_png(bytes: &[u8]) -> Result<Vec<u8>, png::DecodingError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or(png::DecodingError::LimitsExceeded)?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    pixels.truncate(info.buffer_size());
    Ok(match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|[r, g, b]| [*r, *g, *b, u8::MAX])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|[g, a]| [*g, *g, *g, *a])
            .collect(),
        // Grey; a palette is expanded to RGB or RGBA before it gets here.
        png::ColorType::Grayscale | png::ColorType::Indexed => {
            pixels.iter().flat_map(|g| [*g, *g, *g, u8::MAX]).collect()
        }
    })
}

/// The width and height a JPEG's frame header gives, found by walking its
/// segments from the start of the image to the first start-of-frame
/// marker; `None` when the scan or the end comes first, or `bytes` runs
/// out.
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be = |at: usize| {
        let pair = bytes.get(at..at + 2)?.try_into().ok()?;
        Some(u16::from_be_bytes(pair))
    };
    // Past the start-of-image marker.
    let mut at = 2;
    loop {
        // A marker: 0xFF, any fill 0xFFs, then its code.
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        while *bytes.get(at)? == 0xFF {
            at += 1;
        }
        let code = bytes[at];
        at += 1;
        match code {
            // The scan or the end of the image, with no frame before it.
            0xD9 | 0xDA => return None,
            // A start of frame (0xC4, 0xC8 and 0xCC are tables and an
            // extension): its length, its precision, then its height and
            // its width.
            0xC0..=0xCF if !matches!(code, 0xC4 | 0xC8 | 0xCC) => {
                return Some((u32::from(be(at + 5)?), u32::from(be(at + 3)?)));
            }
            // Any other segment: skipped by its length.
            _ => at += usize::from(be(at)?),
        }
    }
}

/// `bytes`, a whole JPEG, as 8-bit RGBA, every alpha 255.
fn decode_jpeg(bytes: &[u8]) -> Result<Vec<u8>, zune_jpeg::errors::DecodeErrors> {
    let options = DecoderOptions::default()
        .set_strict_mode(true)
        .jpeg_set_out_colorspace(ColorSpace::RGBA);
    JpegDecoder::new_with_options(ZCursor::new(bytes), options).decode()
}

#[cfg(test)]
mod tests {
    use nova_galaxy_format::id::Id;

    use super::*;
    use crate::budget::{Limit, Measure};
    use crate::fixture::{JPEG, JPEG_RGB, JPEG_SIZE, PNG, png as rgba_png};

    #[allow(clippy::unnecessary_wraps)]
    fn limit(most: u32) -> Option<Limit> {
        Some(Limit {
            most,
            budget: Id::try_from("default".to_owned()).unwrap(),
        })
    }

    fn side(most: u32) -> Limits {
        Limits {
            side: limit(most),
            ..Limits::default()
        }
    }

    /// A 2 by 1 image: a dark blue pixel, then an orange one.
    const RGBA: [u8; 8] = [10, 20, 30, 255, 200, 100, 50, 128];

    #[test]
    fn a_format_is_sniffed_from_the_first_bytes() {
        assert_eq!(format(&rgba_png(2, 1, &RGBA)), Some(Format::Png));
        assert_eq!(format(PNG), Some(Format::Png), "the signature alone");
        assert_eq!(format(JPEG), Some(Format::Jpeg));
        assert_eq!(format(&[0xFF, 0xD8, 0xFF]), Some(Format::Jpeg));
        assert_eq!(format(b"GIF89a nonsense"), None);
        assert_eq!(format(&[0xFF, 0xD8]), None, "no marker after the start");
        assert_eq!(format(&PNG[..7]), None, "a cut signature");
        assert_eq!(format(&[]), None);
    }

    #[test]
    fn a_png_decodes_to_its_pixels() {
        let bytes = rgba_png(2, 1, &RGBA);
        assert_eq!(
            read_image(&bytes, &Limits::default(), u32::MAX),
            Ok(Pixels {
                width: 2,
                height: 1,
                rgba: RGBA.to_vec(),
            })
        );
    }

    #[test]
    fn a_jpeg_decodes_to_its_colour_with_opaque_alpha() {
        let pixels = read_image(JPEG, &Limits::default(), u32::MAX).unwrap();
        assert_eq!((pixels.width, pixels.height), JPEG_SIZE);
        assert_eq!(JPEG_SIZE, (4, 2), "wider than tall, so a swap shows");
        assert_eq!(pixels.rgba.len(), 4 * 2 * 4);
        for texel in pixels.rgba.as_chunks::<4>().0 {
            let [r, g, b, a] = *texel;
            for (got, want) in [r, g, b].into_iter().zip(JPEG_RGB) {
                assert!(got.abs_diff(want) <= 2, "{texel:?} is not {JPEG_RGB:?}");
            }
            assert_eq!(a, 255);
        }
    }

    #[test]
    fn bytes_that_are_neither_format_are_unknown() {
        for bytes in [&b"GIF89a nonsense"[..], b"", b"glTF"] {
            assert_eq!(
                read_image(bytes, &Limits::default(), u32::MAX),
                Err(NoImage::Unknown)
            );
        }
    }

    /// A PNG's header alone, declaring `width` by `height`.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = rgba_png(2, 1, &RGBA)[..33].to_vec();
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    /// [`JPEG`] with its frame header declaring `width` by `height`: its
    /// data no longer fits, so it can't decode.
    fn jpeg_declaring(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = JPEG.to_vec();
        let frame = bytes
            .windows(2)
            .position(|pair| pair == [0xFF, 0xC0])
            .expect("a baseline frame header");
        bytes[frame + 5..frame + 7].copy_from_slice(&height.to_be_bytes());
        bytes[frame + 7..frame + 9].copy_from_slice(&width.to_be_bytes());
        bytes
    }

    fn over(used: u64, most: u32) -> NoImage {
        NoImage::OverBudget(vec![Overrun {
            measure: Measure::Side,
            used,
            most,
            budget: Id::try_from("default".to_owned()).unwrap(),
        }])
    }

    #[test]
    fn an_image_a_pixel_over_the_budget_s_side_is_refused_from_its_header() {
        // Neither has the pixels its header declares: only the header is
        // read before the budget refuses it.
        for bytes in [png_header(65, 3), png_header(3, 65)] {
            assert_eq!(read_image(&bytes, &side(64), u32::MAX), Err(over(65, 64)));
        }
        for bytes in [jpeg_declaring(65, 3), jpeg_declaring(3, 65)] {
            assert_eq!(read_image(&bytes, &side(64), u32::MAX), Err(over(65, 64)));
        }
    }

    #[test]
    fn an_image_at_the_budget_s_side_is_within_it() {
        let wide = rgba_png(64, 1, &[7; 64 * 4]);
        assert!(read_image(&wide, &side(64), u32::MAX).is_ok());
        let tall = rgba_png(1, 64, &[7; 64 * 4]);
        assert!(read_image(&tall, &side(64), u32::MAX).is_ok());
        assert!(read_image(JPEG, &side(4), u32::MAX).is_ok());
        assert_eq!(read_image(JPEG, &side(3), u32::MAX), Err(over(4, 3)));
    }

    #[test]
    fn the_budget_s_kib_and_triangles_never_refuse_an_image() {
        // Its bytes are its model's to count.
        let limits = Limits {
            triangles: limit(0),
            kib: limit(0),
            side: None,
        };
        assert!(read_image(&rgba_png(2, 1, &RGBA), &limits, u32::MAX).is_ok());
        assert!(read_image(JPEG, &limits, u32::MAX).is_ok());
    }

    #[test]
    fn an_image_wider_than_the_caller_draws_is_refused_from_its_header() {
        for bytes in [
            png_header(8_193, 2),
            png_header(2, 8_193),
            jpeg_declaring(8_193, 2),
            jpeg_declaring(2, 8_193),
        ] {
            assert_eq!(
                read_image(&bytes, &Limits::default(), 8_192),
                Err(NoImage::TooLarge {
                    side: 8_193,
                    most: 8_192,
                })
            );
        }
        // At the most is within it: each fails only when it decodes.
        for bytes in [png_header(8_192, 2), jpeg_declaring(8_192, 2)] {
            assert!(matches!(
                read_image(&bytes, &Limits::default(), 8_192),
                Err(NoImage::Undecodable(_))
            ));
        }
        assert!(read_image(JPEG, &Limits::default(), 4).is_ok());
        assert!(read_image(&rgba_png(2, 1, &RGBA), &Limits::default(), 2).is_ok());
    }

    #[test]
    fn the_budget_is_checked_before_the_caller_s_side() {
        assert_eq!(
            read_image(&png_header(65, 3), &side(64), 8),
            Err(over(65, 64))
        );
    }

    #[test]
    fn a_header_that_gives_no_size_does_not_decode() {
        let no_size = Err(NoImage::Undecodable("its header gives no size".to_owned()));
        // A PNG's signature without its header chunk.
        let mut headless = rgba_png(2, 1, &RGBA);
        headless[12..16].copy_from_slice(b"IDAT");
        assert_eq!(read_image(&headless, &Limits::default(), u32::MAX), no_size);
        // The fixture's stub: a signature and the start of a header.
        assert_eq!(read_image(PNG, &Limits::default(), u32::MAX), no_size);
        // A JPEG that ends before its frame header.
        assert_eq!(
            read_image(&JPEG[..40], &Limits::default(), u32::MAX),
            no_size
        );
    }

    #[test]
    fn a_truncated_image_does_not_decode() {
        // Each keeps its whole header, so its size reads, and loses the
        // end of its data.
        let png = rgba_png(64, 32, &vec![9; 64 * 32 * 4]);
        for bytes in [&png[..png.len() / 2], &JPEG[..JPEG.len() - 12]] {
            let Err(NoImage::Undecodable(why)) = read_image(bytes, &Limits::default(), u32::MAX)
            else {
                panic!("it decoded");
            };
            assert_ne!(why, "its header gives no size");
            assert!(!why.is_empty());
        }
    }

    /// A JPEG's start, then `segments`.
    fn jpeg_of(segments: &[&[u8]]) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8];
        for segment in segments {
            bytes.extend_from_slice(segment);
        }
        bytes
    }

    /// A start-of-frame segment of marker `code`, 0x0102 wide and 0x0304
    /// tall.
    fn frame(code: u8) -> Vec<u8> {
        vec![0xFF, code, 0x00, 0x08, 0x08, 0x03, 0x04, 0x01, 0x02, 0x01]
    }

    /// A segment of marker `code` holding `body`.
    fn segment(code: u8, body: &[u8]) -> Vec<u8> {
        let length = u16::try_from(body.len() + 2).unwrap().to_be_bytes();
        [&[0xFF, code][..], &length, body].concat()
    }

    #[test]
    fn a_jpeg_s_size_is_its_first_frame_header_s_width_then_height() {
        assert_eq!(jpeg_size(JPEG), Some((4, 2)));
        let size = Some((0x0102, 0x0304));
        // Baseline, progressive, and the last frame marker.
        for code in [0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xCF] {
            assert_eq!(jpeg_size(&jpeg_of(&[&frame(code)])), size, "{code:#x}");
        }
        // After other segments, each skipped by its length: a Huffman
        // table (0xC4), an arithmetic conditioning (0xCC) and the JPEG
        // extension (0xC8) are no frame, though numbered among them.
        for code in [0xE0, 0xDB, 0xC4, 0xC8, 0xCC, 0xFE] {
            let before = segment(code, &[0xFF, 0xC0, 9, 9, 9, 9, 9, 9, 9]);
            let bytes = jpeg_of(&[&before, &frame(0xC0)]);
            assert_eq!(jpeg_size(&bytes), size, "{code:#x}");
        }
        // Fill bytes before a marker's code.
        let filled = jpeg_of(&[&[0xFF, 0xFF], &frame(0xC2)]);
        assert_eq!(jpeg_size(&filled), size);
    }

    #[test]
    fn a_jpeg_with_no_frame_header_before_its_scan_has_no_size() {
        // The scan, then a frame: too late.
        let scan = jpeg_of(&[&segment(0xDA, &[1, 2]), &frame(0xC0)]);
        assert_eq!(jpeg_size(&scan), None);
        let ended = jpeg_of(&[&[0xFF, 0xD9], &frame(0xC0)]);
        assert_eq!(jpeg_size(&ended), None);
        // Not a marker where one must be.
        assert_eq!(jpeg_size(&jpeg_of(&[&[0x00], &frame(0xC0)])), None);
        // Cut short: in a marker, in a segment's length, in the frame.
        assert_eq!(jpeg_size(&[0xFF, 0xD8]), None);
        assert_eq!(jpeg_size(&[0xFF, 0xD8, 0xFF]), None);
        assert_eq!(jpeg_size(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]), None);
        let whole = jpeg_of(&[&frame(0xC0)]);
        assert_eq!(jpeg_size(&whole[..whole.len() - 2]), None);
        assert_eq!(jpeg_size(&whole[..whole.len() - 1]), size_of_whole());
        // A segment whose length runs past the end.
        assert_eq!(jpeg_size(&jpeg_of(&[&[0xFF, 0xE0, 0x00, 0x40]])), None);
        // A length of nothing never loops: what follows is no marker.
        assert_eq!(jpeg_size(&jpeg_of(&[&[0xFF, 0xE0, 0x00, 0x00]])), None);
    }

    /// The size [`frame`] declares.
    #[allow(clippy::unnecessary_wraps)]
    fn size_of_whole() -> Option<(u32, u32)> {
        Some((0x0102, 0x0304))
    }

    #[test]
    fn a_png_s_size_is_its_header_s() {
        assert_eq!(png_size(&rgba_png(4, 2, &[0; 32])), Some((4, 2)));
        assert_eq!(png_size(&png_header(7, 9)), Some((7, 9)));
        assert_eq!(png_size(&png_header(7, 9)[..23]), None, "cut in its height");
        assert_eq!(png_size(JPEG), None);
    }
}
