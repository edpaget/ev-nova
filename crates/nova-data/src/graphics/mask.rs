//! Masking one decoded picture with another.

use super::error::GraphicsError;
use super::image::Image;

/// The luminance below which a mask pixel is dark.
const DARK_BELOW: u32 = 128;

/// `image` with its alpha taken from `mask`, as a `cicn`'s mask works:
/// where the mask is dark (luminance below 128) the image's pixel is kept,
/// and where it is light the pixel is fully transparent black. Nova's
/// masks are black where the picture shows and white where it does not.
/// The mask's own alpha is ignored.
///
/// The two must be the same size ([`GraphicsError::MaskSizeMismatch`]).
pub fn apply_mask(image: &Image, mask: &Image) -> Result<Image, GraphicsError> {
    if (image.width(), image.height()) != (mask.width(), mask.height()) {
        return Err(GraphicsError::MaskSizeMismatch {
            width: image.width(),
            height: image.height(),
            mask_width: mask.width(),
            mask_height: mask.height(),
        });
    }
    let pixels = image
        .pixels()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(mask.pixels().as_chunks::<4>().0)
        .flat_map(|(&pixel, mask)| {
            if luminance(*mask) < DARK_BELOW {
                pixel
            } else {
                [0; 4]
            }
        })
        .collect();
    Ok(Image::from_rgba(image.width(), image.height(), pixels).expect("same size as the image"))
}

/// A pixel's luminance (ITU-R BT.601 weights), 0 to 255.
fn luminance(rgba: [u8; 4]) -> u32 {
    let [r, g, b, _] = rgba.map(u32::from);
    (299 * r + 587 * g + 114 * b) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` x 1 image of these pixels.
    fn row(pixels: &[[u8; 4]]) -> Image {
        Image::from_rgba(pixels.len() as u32, 1, pixels.concat()).expect("row")
    }

    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const CLEAR: [u8; 4] = [0; 4];

    fn grey(level: u8) -> [u8; 4] {
        [level, level, level, 255]
    }

    #[test]
    fn black_keeps_the_pixel_and_white_clears_it() {
        let masked = apply_mask(&row(&[RED, BLUE, RED]), &row(&[BLACK, WHITE, BLACK]));
        assert_eq!(masked, Ok(row(&[RED, CLEAR, RED])));
    }

    #[test]
    fn the_image_s_own_alpha_is_kept_under_black() {
        let half = [10, 20, 30, 128];
        assert_eq!(apply_mask(&row(&[half]), &row(&[BLACK])), Ok(row(&[half])));
    }

    #[test]
    fn grey_below_128_is_dark_and_from_128_light() {
        let masked = apply_mask(
            &row(&[RED, RED, RED, RED]),
            &row(&[grey(0), grey(127), grey(128), grey(255)]),
        );
        assert_eq!(masked, Ok(row(&[RED, RED, CLEAR, CLEAR])));
    }

    #[test]
    fn luminance_weights_green_most_and_blue_least() {
        // Pure green is 149 (light), pure red 76 and pure blue 29 (dark).
        let masked = apply_mask(
            &row(&[BLUE, BLUE, BLUE]),
            &row(&[[0, 255, 0, 255], RED, BLUE]),
        );
        assert_eq!(masked, Ok(row(&[CLEAR, BLUE, BLUE])));
        assert_eq!(luminance([0, 255, 0, 255]), 149);
        assert_eq!(luminance(RED), 76);
        assert_eq!(luminance(BLUE), 29);
        assert_eq!(luminance(WHITE), 255);
        assert_eq!(luminance([0, 218, 0, 255]), 127);
        assert_eq!(luminance([0, 219, 0, 255]), 128);
    }

    #[test]
    fn the_mask_s_alpha_is_ignored() {
        let masked = apply_mask(&row(&[RED, RED]), &row(&[[0, 0, 0, 0], [255, 255, 255, 0]]));
        assert_eq!(masked, Ok(row(&[RED, CLEAR])));
    }

    #[test]
    fn a_mask_of_another_size_is_an_error() {
        let image = Image::from_rgba(2, 1, [RED, RED].concat()).expect("image");
        let tall = Image::from_rgba(2, 2, [BLACK; 4].concat()).expect("mask");
        assert_eq!(
            apply_mask(&image, &tall),
            Err(GraphicsError::MaskSizeMismatch {
                width: 2,
                height: 1,
                mask_width: 2,
                mask_height: 2,
            })
        );
        let wide = row(&[BLACK; 3]);
        assert_eq!(
            apply_mask(&image, &wide),
            Err(GraphicsError::MaskSizeMismatch {
                width: 2,
                height: 1,
                mask_width: 3,
                mask_height: 1,
            })
        );
    }
}
