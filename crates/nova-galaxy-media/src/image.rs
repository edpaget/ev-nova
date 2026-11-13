//! An image read as pixels: a PNG or a JPEG, decoded to 8-bit RGBA.
//!
//! [`read_image`] reads one of a model's images (the renderer's model
//! adapter, `nova_render::GalaxyModels`, hands it each image a material's
//! base colour names). It checks what the image may cost before it
//! decodes it, in [`read_texture`](crate::texture::read_texture)'s order:
//!
//! 1. which format it is, sniffed from its first bytes ([`format()`]), not
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
//! The size checked is the size decoded. A file's author chooses its
//! bytes, so a header read one way to size the image and another way to
//! decode it could pass the limits as a small image and decode as a large
//! one. A PNG's header is its first chunk, at a fixed place, for this
//! module and for its decoder alike. A JPEG's frame header is wherever
//! its segments put it, so its decoder alone reads it: the decoder reads
//! the headers, gives the size, and after the checks goes on to the
//! pixels from where it stopped. Either way, pixels that aren't the
//! header's width by its height are refused.
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
    /// gives no size, its data is cut short or corrupt, or it decodes to
    /// another size than its header's. Why.
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
    match format(bytes).ok_or(NoImage::Unknown)? {
        Format::Png => {
            let (width, height) =
                png_size(bytes).ok_or_else(|| NoImage::Undecodable(NO_SIZE.to_owned()))?;
            affordable(width, height, limits, max_side)?;
            let rgba = decode_png(bytes).map_err(|e| NoImage::Undecodable(e.to_string()))?;
            pixels(width, height, rgba)
        }
        Format::Jpeg => {
            let (mut jpeg, (width, height)) = jpeg_headers(bytes)
                .map_err(|why| NoImage::Undecodable(format!("{NO_SIZE}: {why}")))?;
            affordable(width, height, limits, max_side)?;
            let rgba = jpeg
                .decode()
                .map_err(|e| NoImage::Undecodable(e.to_string()))?;
            pixels(width, height, rgba)
        }
    }
}

/// Why an image whose header doesn't size it doesn't decode. A JPEG's
/// adds its decoder's reason after it.
const NO_SIZE: &str = "its header gives no size";

/// Holds an image `width` by `height`, as its header sizes it, to
/// `limits`' `side`, then to `max_side`.
fn affordable(width: u32, height: u32, limits: &Limits, max_side: u32) -> Result<(), NoImage> {
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
    Ok(())
}

/// `rgba` as the pixels of the image its header sized `width` by
/// `height`. Refused when it isn't four bytes for each of them: what
/// decoded is some other size than the one the limits were checked for.
fn pixels(width: u32, height: u32, rgba: Vec<u8>) -> Result<Pixels, NoImage> {
    if u64::from(width) * u64::from(height) * 4 != rgba.len() as u64 {
        return Err(NoImage::Undecodable(format!(
            "it decoded to {} bytes, not the {width} by {height} pixels its header gives",
            rgba.len()
        )));
    }
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

/// A JPEG's decoder over its bytes.
type Jpeg<'a> = JpegDecoder<ZCursor<&'a [u8]>>;

/// The decoder of `bytes`, a JPEG, with its headers read as far as its
/// scan and no pixel decoded, and the width and height its frame header
/// gives; or why its headers don't read.
///
/// The decoder that sizes the image is the one that then decodes it
/// ([`JpegDecoder::decode`] goes on from the scan, to 8-bit RGBA with
/// every alpha 255), so the size the limits are checked for is the size
/// decoded: no second reading of the headers can take another segment for
/// the frame. The decoder refuses a second frame header, so the size
/// can't change after it is checked either.
///
/// The decoder's own largest width and height are raised to the most a
/// frame header can declare, so that a large image is refused by the
/// caller's limits, with their reasons, not as one that doesn't read.
fn jpeg_headers(bytes: &[u8]) -> Result<(Jpeg<'_>, (u32, u32)), String> {
    let most = usize::from(u16::MAX);
    let options = DecoderOptions::default()
        .set_strict_mode(true)
        .jpeg_set_out_colorspace(ColorSpace::RGBA)
        .set_max_width(most)
        .set_max_height(most);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions().ok_or("it has no frame")?;
    let size = u32::try_from(width).ok().zip(u32::try_from(height).ok());
    Ok((decoder, size.ok_or("its frame is too large")?))
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

    /// [`JPEG`] with a decoy frame header declaring 1 by 1 hidden before
    /// its own. After the start of the image comes `FF 00`, which a
    /// decoder takes for a stuffed byte and reads past, to the `APP0`
    /// segment that follows: 512 bytes long, the decoy inside it, and
    /// after it the rest of the fixture with its real 4 by 2 frame. A
    /// walk that took `FF 00` for a marker would skip by the "length"
    /// after it (`00 FF`, 255) and land on the decoy at offset 259.
    fn jpeg_with_a_decoy_frame() -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0x00, 0x00, 0xFF, 0xE0, 0x02, 0x00];
        // The segment runs 512 bytes from its length, at offset 7.
        bytes.resize(7 + 0x200, 0);
        bytes[259..268].copy_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01]);
        bytes.extend_from_slice(&JPEG[2..]);
        bytes
    }

    #[test]
    fn a_jpeg_is_held_to_the_frame_it_decodes_not_to_a_decoy_before_it() {
        let bytes = jpeg_with_a_decoy_frame();
        // The decoy's 1 by 1 is within each limit; the frame's 4 by 2 is
        // over them.
        assert_eq!(read_image(&bytes, &side(3), u32::MAX), Err(over(4, 3)));
        assert_eq!(
            read_image(&bytes, &Limits::default(), 3),
            Err(NoImage::TooLarge { side: 4, most: 3 })
        );
        // Within them, it is the size it decodes to.
        let pixels = read_image(&bytes, &side(4), 4).unwrap();
        assert_eq!((pixels.width, pixels.height), JPEG_SIZE);
        assert_eq!(pixels.rgba.len(), 4 * 2 * 4);
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
    }

    /// Why `bytes`, a JPEG, has no size: its decoder's reason, after
    /// "its header gives no size".
    #[track_caller]
    fn jpeg_no_size(bytes: &[u8]) -> String {
        let Err(NoImage::Undecodable(why)) = read_image(bytes, &Limits::default(), u32::MAX) else {
            panic!("its headers read");
        };
        let reason = why.strip_prefix("its header gives no size: ");
        let reason = reason.unwrap_or_else(|| panic!("it was sized, and then: {why}"));
        assert!(!reason.is_empty());
        reason.to_owned()
    }

    /// Where [`JPEG`]'s frame header (`SOF0`) starts, and its length with
    /// its marker.
    fn frame_header() -> (usize, usize) {
        let at = JPEG
            .windows(2)
            .position(|pair| pair == [0xFF, 0xC0])
            .expect("a baseline frame header");
        let length = u16::from_be_bytes([JPEG[at + 2], JPEG[at + 3]]);
        (at, 2 + usize::from(length))
    }

    #[test]
    fn a_jpeg_whose_headers_do_not_read_has_no_size_and_says_why() {
        // The start of the image and no more.
        jpeg_no_size(&[0xFF, 0xD8, 0xFF]);
        // Cut before its frame header, and inside it.
        let (at, length) = frame_header();
        jpeg_no_size(&JPEG[..at]);
        jpeg_no_size(&JPEG[..at + length - 1]);
        // Cut after its frame header, before its scan: the frame's size
        // was read, and the headers still don't.
        jpeg_no_size(&JPEG[..at + length]);
        // No frame header before its scan.
        let frameless = [&JPEG[..at], &JPEG[at + length..]].concat();
        jpeg_no_size(&frameless);
        // A kind of frame the decoder doesn't decode (lossless, `SOF3`)
        // is no frame to it, so it isn't one to size the image by.
        let mut lossless = JPEG.to_vec();
        lossless[at + 1] = 0xC3;
        jpeg_no_size(&lossless);
    }

    #[test]
    fn a_jpeg_with_a_second_frame_header_is_refused_before_it_decodes() {
        // The fixture's frame header twice over, the second declaring a
        // larger frame: the size checked can't be the first's and the
        // size decoded the second's.
        let (at, length) = frame_header();
        let mut second = JPEG[at..at + length].to_vec();
        second[5..9].copy_from_slice(&[0x10, 0x00, 0x10, 0x00]);
        let bytes = [&JPEG[..at + length], &second[..], &JPEG[at + length..]].concat();
        assert!(jpeg_no_size(&bytes).contains("Two Start of Frame"));
    }

    #[test]
    fn a_jpeg_of_any_size_its_header_can_declare_is_held_to_the_caller_s_limits() {
        // Larger than the decoder's own default allows: still the
        // budget's and the caller's to refuse, with their reasons.
        for bytes in [jpeg_declaring(u16::MAX, 2), jpeg_declaring(2, u16::MAX)] {
            assert_eq!(
                read_image(&bytes, &side(64), u32::MAX),
                Err(over(65_535, 64))
            );
            assert_eq!(
                read_image(&bytes, &Limits::default(), 8_192),
                Err(NoImage::TooLarge {
                    side: 65_535,
                    most: 8_192,
                })
            );
        }
    }

    #[test]
    fn pixels_that_are_not_their_header_s_size_are_refused() {
        assert_eq!(
            pixels(4, 2, vec![7; 32]),
            Ok(Pixels {
                width: 4,
                height: 2,
                rgba: vec![7; 32],
            })
        );
        // The decoy's case: sized 1 by 1, decoded 4 by 2.
        for (width, height, bytes) in [(1, 1, 32), (4, 2, 31), (4, 2, 33), (2, 4, 8), (4, 2, 0)] {
            let Err(NoImage::Undecodable(why)) = pixels(width, height, vec![7; bytes]) else {
                panic!("{width} by {height} took {bytes} bytes");
            };
            assert!(
                why.contains(&format!("{bytes} bytes"))
                    && why.contains(&format!("{width} by {height}")),
                "{why}"
            );
        }
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

    #[test]
    fn a_png_s_size_is_its_header_s() {
        assert_eq!(png_size(&rgba_png(4, 2, &[0; 32])), Some((4, 2)));
        assert_eq!(png_size(&png_header(7, 9)), Some((7, 9)));
        assert_eq!(png_size(&png_header(7, 9)[..23]), None, "cut in its height");
        assert_eq!(png_size(JPEG), None);
    }
}
