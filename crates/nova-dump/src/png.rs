//! Images as PNG files.
//!
//! Pixels are written as 8-bit RGBA with straight alpha, exactly as
//! [`Image`] holds them, and compressed with the encoder's fast setting:
//! on the stock data it is about ten times faster than the default for
//! about two thirds more bytes, and a dump is for browsing, not shipping.

use nova_data::graphics::Image;

pub use ::png::EncodingError;

/// The image as a PNG file. An empty image (a zero width or height) is an
/// error: PNG cannot hold one. The decoders never produce one.
pub fn encode(image: &Image) -> Result<Vec<u8>, EncodingError> {
    let mut out = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut out, image.width(), image.height());
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    encoder.set_compression(::png::Compression::Fast);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(image.pixels())?;
    writer.finish()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decodes a PNG back to (width, height, RGBA8 pixels).
    fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = ::png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("a PNG header");
        let mut pixels = vec![0; reader.output_buffer_size().expect("size")];
        let info = reader.next_frame(&mut pixels).expect("a frame");
        assert_eq!(info.color_type, ::png::ColorType::Rgba);
        assert_eq!(info.bit_depth, ::png::BitDepth::Eight);
        pixels.truncate(info.buffer_size());
        (info.width, info.height, pixels)
    }

    #[test]
    fn an_image_round_trips_with_its_alpha() {
        #[rustfmt::skip]
        let pixels = vec![
            255, 0, 0, 255,    0, 255, 0, 128,   0, 0, 255, 0,
            1, 2, 3, 4,        5, 6, 7, 8,       9, 10, 11, 12,
        ];
        let image = Image::from_rgba(3, 2, pixels.clone()).expect("3x2");
        assert_eq!(decode(&encode(&image).expect("encodes")), (3, 2, pixels));
    }

    #[test]
    fn large_flat_images_compress() {
        let image = Image::from_rgba(640, 480, vec![7; 640 * 480 * 4]).expect("640x480");
        let png = encode(&image).expect("encodes");
        assert!(png.len() < 640 * 480 / 10, "{} bytes", png.len());
        assert_eq!(decode(&png).2, image.pixels());
    }

    #[test]
    fn the_pixels_are_compressed_for_speed() {
        let image = Image::from_rgba(2, 2, vec![9; 16]).expect("2x2");
        let png = encode(&image).expect("encodes");
        let idat = png
            .windows(4)
            .position(|w| w == b"IDAT")
            .expect("an IDAT chunk");
        // The zlib header's FLEVEL (the top two bits of its second byte)
        // says how hard the compressor tried: 0 is the fastest.
        let (cmf, flg) = (png[idat + 4], png[idat + 5]);
        assert_eq!(cmf & 0x0F, 8, "deflate");
        assert_eq!(flg >> 6, 0, "fastest level, got {flg:#04x}");
    }

    #[test]
    fn an_empty_image_is_an_error() {
        let image = Image::from_rgba(0, 5, Vec::new()).expect("0x5");
        assert!(encode(&image).is_err());
    }
}
