//! Decoded RGBA8 images.

use std::fmt;

/// An RGBA8 image: row-major, top row first, four bytes per pixel, straight
/// (not premultiplied) alpha.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    /// Wraps RGBA8 pixel bytes, or `None` unless there are exactly
    /// `width * height * 4` of them.
    #[must_use]
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let len = u128::from(width) * u128::from(height) * 4;
        (u128::try_from(pixels.len()) == Ok(len)).then_some(Self {
            width,
            height,
            pixels,
        })
    }

    /// A fully transparent image. Callers check the size against the
    /// decoding budget first.
    pub(crate) fn transparent(width: u32, height: u32) -> Self {
        let len = width as usize * height as usize * 4;
        Self {
            width,
            height,
            pixels: vec![0; len],
        }
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The RGBA8 bytes.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The RGBA8 bytes, by value.
    #[must_use]
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    /// The pixel at (`x`, `y`), or `None` outside the image.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let start = self.index(x, y);
        self.pixels.get(start..start + 4)?.try_into().ok()
    }

    /// Sets the pixel at (`x`, `y`); callers keep it inside the image.
    pub(crate) fn put(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let start = self.index(x, y);
        if let Some(pixel) = self.pixels.get_mut(start..start + 4) {
            pixel.copy_from_slice(&rgba);
        }
    }

    /// Copies `src` onto this image with its top-left corner at (`x`, `y`);
    /// callers keep it inside the image.
    pub(crate) fn blit(&mut self, src: &Image, x: u32, y: u32) {
        let len = src.width as usize * 4;
        for (row, line) in (0..src.height).zip(src.pixels.chunks(len)) {
            let start = self.index(x, y + row);
            if let Some(dst) = self.pixels.get_mut(start..start + len) {
                dst.copy_from_slice(line);
            }
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
    }
}

/// Prints the size only, so failed assertions on big images stay readable.
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.pixels.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_by_three() -> Image {
        Image::from_rgba(2, 3, (0..24).collect()).unwrap()
    }

    #[test]
    fn from_rgba_needs_exactly_four_bytes_per_pixel() {
        assert!(Image::from_rgba(2, 3, vec![0; 23]).is_none());
        assert!(Image::from_rgba(2, 3, vec![0; 25]).is_none());
        assert!(Image::from_rgba(0, 0, Vec::new()).is_some());
        assert!(Image::from_rgba(u32::MAX, u32::MAX, Vec::new()).is_none());
        let image = two_by_three();
        assert_eq!((image.width(), image.height()), (2, 3));
        assert_eq!(image.pixels(), (0..24).collect::<Vec<u8>>());
        assert_eq!(image.into_pixels(), (0..24).collect::<Vec<u8>>());
    }

    #[test]
    fn pixels_are_row_major_rgba() {
        let image = two_by_three();
        assert_eq!(image.pixel(0, 0), Some([0, 1, 2, 3]));
        assert_eq!(image.pixel(1, 0), Some([4, 5, 6, 7]));
        assert_eq!(image.pixel(0, 1), Some([8, 9, 10, 11]));
        assert_eq!(image.pixel(1, 2), Some([20, 21, 22, 23]));
    }

    #[test]
    fn pixels_outside_the_image_are_none() {
        let image = two_by_three();
        assert_eq!(image.pixel(2, 0), None);
        assert_eq!(image.pixel(0, 3), None);
        // Inside the buffer (it is row 1's pixel), but past the end of row 0.
        let tall = Image::from_rgba(1, 3, vec![0; 12]).unwrap();
        assert_eq!(tall.pixel(1, 0), None);
    }

    #[test]
    fn put_sets_one_pixel_of_a_transparent_image() {
        let mut image = Image::transparent(3, 2);
        assert_eq!(image.pixels(), &[0; 24][..]);
        image.put(2, 1, [1, 2, 3, 4]);
        image.put(0, 1, [5, 6, 7, 8]);
        assert_eq!(image.pixel(2, 1), Some([1, 2, 3, 4]));
        assert_eq!(image.pixel(0, 1), Some([5, 6, 7, 8]));
        assert_eq!(image.pixel(1, 1), Some([0; 4]));
        assert_eq!(image.pixels().iter().filter(|&&b| b != 0).count(), 8);
    }

    #[test]
    fn blit_copies_every_row_to_its_place() {
        let src = Image::from_rgba(2, 2, (1..=16).collect()).unwrap();
        let mut dst = Image::transparent(4, 3);
        dst.blit(&src, 1, 1);
        assert_eq!(dst.pixel(0, 1), Some([0; 4]));
        assert_eq!(dst.pixel(1, 1), Some([1, 2, 3, 4]));
        assert_eq!(dst.pixel(2, 1), Some([5, 6, 7, 8]));
        assert_eq!(dst.pixel(1, 2), Some([9, 10, 11, 12]));
        assert_eq!(dst.pixel(2, 2), Some([13, 14, 15, 16]));
        assert_eq!(dst.pixel(3, 2), Some([0; 4]));
        assert_eq!(dst.pixels().iter().filter(|&&b| b != 0).count(), 16);
        let mut corner = Image::transparent(2, 2);
        corner.blit(&src, 0, 0);
        assert_eq!(corner, src);
    }

    #[test]
    fn debug_shows_the_size_not_the_pixels() {
        assert_eq!(
            format!("{:?}", two_by_three()),
            "Image { width: 2, height: 3, bytes: 24 }"
        );
    }
}
