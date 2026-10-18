//! The main screen's art, in the core's terms, and the port it comes
//! through ([`MainScreenSource`]).
//!
//! `_LoadMainScreenAnim` @0x1781d and `_DoMainScreen` load it from the
//! game data: `PICT` 8000 behind everything, the six buttons' `spïn` 600
//! to 605 (each an `rlëD` whose frame 0 is up and frame 1 down), the
//! logo's `spïn` 606 (a `PICT` of frames), the rollover's `spïn` 607 (an
//! `rlëD`, a frame per button and a last one for none), the three slides'
//! `spïn` 608 to 610 (each a `PICT` of frames, a row of buttons sliding
//! in, `_GetSpriteInfo(608 + i)` @0x178fa), and their positions from
//! `cölr` 128, offsets from `menuTopCorner`
//! ([`interface_background`](super::background::interface_background)).
//!
//! The buttons are hit-tested per pixel against frame 0's mask:
//! `_SWRLECollisionWithPoint` @0xac1ab tests a sprite's current frame,
//! and nothing on the main screen changes a button's current frame from
//! 0 (the pressed look is blitted straight from frame 1). So only frame
//! 0's mask is kept.

use std::rc::Rc;

use nova_data::graphics::Image;

use crate::geometry::Point;

/// The main screen's picture.
pub const BACKGROUND_PICT: i16 = 8000;
/// The buttons' `spïn`s, New Pilot's first.
pub const BUTTON_SPINS: [i16; 6] = [600, 601, 602, 603, 604, 605];
/// The logo's `spïn`.
pub const LOGO_SPIN: i16 = 606;
/// The rollover's `spïn`.
pub const ROLLOVER_SPIN: i16 = 607;
/// The slides' `spïn`s, the top row's first.
pub const SLIDE_SPINS: [i16; 3] = [608, 609, 610];
/// The `cölr` the positions come from.
pub const COLORS_ID: i16 = 128;

/// Which pixels of a sprite frame are opaque: its alpha as a mask.
#[derive(Clone, PartialEq, Eq)]
pub struct SpriteMask {
    width: u32,
    height: u32,
    opaque: Vec<bool>,
}

impl std::fmt::Debug for SpriteMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpriteMask")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl SpriteMask {
    /// A `width` x `height` mask, row by row, or `None` unless `opaque`
    /// holds exactly `width * height` pixels.
    #[must_use]
    pub fn new(width: u32, height: u32, opaque: Vec<bool>) -> Option<Self> {
        let count = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        (opaque.len() == count).then_some(Self {
            width,
            height,
            opaque,
        })
    }

    /// `image`'s mask: opaque where its alpha is above 0. A decoded
    /// `rlëD` frame is straight-alpha, so its alpha is its mask.
    #[must_use]
    pub fn from_alpha(image: &Image) -> Self {
        let (pixels, _) = image.pixels().as_chunks::<4>();
        let opaque = pixels.iter().map(|pixel| pixel[3] > 0).collect();
        Self {
            width: image.width(),
            height: image.height(),
            opaque,
        }
    }

    /// The mask's width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The mask's height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether the pixel at (`x`, `y`) is opaque; nothing outside the
    /// mask is.
    #[must_use]
    pub fn opaque_at(&self, x: i64, y: i64) -> bool {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return false;
        };
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = u64::from(y) * u64::from(self.width) + u64::from(x);
        usize::try_from(index)
            .ok()
            .and_then(|index| self.opaque.get(index))
            .copied()
            .unwrap_or(false)
    }

    /// An opaque pixel: the centre, when it is opaque, or else the first
    /// in row order; `None` when the mask is clear.
    #[must_use]
    pub fn opaque_pixel(&self) -> Option<(u32, u32)> {
        let center = (self.width / 2, self.height / 2);
        if self.opaque_at(i64::from(center.0), i64::from(center.1)) {
            return Some(center);
        }
        let width = usize::try_from(self.width).ok()?.max(1);
        let index = self.opaque.iter().position(|&opaque| opaque)?;
        let x = u32::try_from(index % width).ok()?;
        let y = u32::try_from(index / width).ok()?;
        Some((x, y))
    }
}

/// One of the six buttons: `rlëD` `sprite`, frame 0 up and frame 1 down,
/// drawn at `position` from `menuTopCorner`.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonArt {
    /// The `rlëD`'s ID.
    pub sprite: i16,
    /// Where its top-left goes, from `menuTopCorner` (`cölr` 128's
    /// `Button1x & y` to `Button6x & y`).
    pub position: Point,
    /// Frame 0's mask, which a press is hit-tested on.
    pub mask: SpriteMask,
}

/// A `PICT` of frames, the logo or a slide: `PICT` `picture`, a grid of
/// `frames` frames of `frame_size`, `columns` across, drawn at `position`
/// from `menuTopCorner`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictFrames {
    /// The `PICT`'s ID.
    pub picture: i16,
    /// Each frame's width and height.
    pub frame_size: (f32, f32),
    /// The frames across the picture.
    pub columns: u16,
    /// How many frames there are.
    pub frames: u16,
    /// Where a frame's top-left goes, from `menuTopCorner` (`cölr` 128's
    /// `LogoX & Y`, or a slide's `Slide1x & y` to `Slide3x & y`).
    pub position: Point,
}

impl PictFrames {
    /// Where frame `frame`'s top-left is in the picture.
    #[must_use]
    pub fn frame_origin(&self, frame: u16) -> Point {
        let columns = self.columns.max(1);
        Point::new(
            f32::from(frame % columns) * self.frame_size.0,
            f32::from(frame / columns) * self.frame_size.1,
        )
    }
}

/// The rollover: `rlëD` `sprite`, a frame per button and a last one for
/// none, drawn at `position` from `menuTopCorner`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RolloverArt {
    /// The `rlëD`'s ID.
    pub sprite: i16,
    /// Each frame's width and height.
    pub frame_size: (f32, f32),
    /// How many frames there are.
    pub frames: u16,
    /// Where its top-left goes, from `menuTopCorner` (`cölr` 128's
    /// `RolloverX & Y`).
    pub position: Point,
}

/// Everything the main screen draws.
#[derive(Clone, Debug, PartialEq)]
pub struct MainScreenArt {
    /// The `PICT` behind everything.
    pub background: i16,
    /// Its width and height.
    pub background_size: (u32, u32),
    /// The six buttons, New Pilot's first, in `_MainScreenChoice`'s
    /// order.
    pub buttons: [ButtonArt; 6],
    /// The logo.
    pub logo: PictFrames,
    /// The three slides, the top row's first: the rows of buttons sliding
    /// in as the screen appears.
    pub slides: [PictFrames; 3],
    /// The rollover.
    pub rollover: RolloverArt,
}

/// Where the main screen's art comes from.
pub trait MainScreenSource {
    /// The main screen's art, or why it cannot be drawn: a message naming
    /// what is missing or does not decode.
    fn main_screen_art(&self) -> Result<MainScreenArt, String>;
}

/// A borrowed source is a source.
impl<T: MainScreenSource + ?Sized> MainScreenSource for &T {
    fn main_screen_art(&self) -> Result<MainScreenArt, String> {
        (**self).main_screen_art()
    }
}

/// A shared source is a source, so the router can read the game data it
/// holds.
impl<T: MainScreenSource + ?Sized> MainScreenSource for Rc<T> {
    fn main_screen_art(&self) -> Result<MainScreenArt, String> {
        (**self).main_screen_art()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 x 2 mask, opaque only at (1, 0) and (2, 1).
    fn mask() -> SpriteMask {
        SpriteMask::new(3, 2, vec![false, true, false, false, false, true]).expect("6 pixels")
    }

    #[test]
    fn a_mask_needs_a_pixel_for_each_place() {
        assert!(SpriteMask::new(3, 2, vec![true; 5]).is_none());
        assert!(SpriteMask::new(3, 2, vec![true; 7]).is_none());
        let full = SpriteMask::new(3, 2, vec![true; 6]).expect("6 pixels");
        assert_eq!((full.width(), full.height()), (3, 2));
    }

    #[test]
    fn a_mask_is_opaque_only_at_its_opaque_pixels_and_never_outside() {
        let mask = mask();
        let opaque: Vec<(i64, i64)> = (-1..4)
            .flat_map(|y| (-1..5).map(move |x| (x, y)))
            .filter(|&(x, y)| mask.opaque_at(x, y))
            .collect();
        assert_eq!(opaque, [(1, 0), (2, 1)]);
        // Past the right edge is outside, not the next row's start.
        let next_row =
            SpriteMask::new(3, 2, vec![false, false, false, true, false, false]).expect("6 pixels");
        assert!(!next_row.opaque_at(3, 0));
        assert!(next_row.opaque_at(0, 1));
    }

    #[test]
    fn debug_shows_a_masks_size() {
        assert_eq!(
            format!("{:?}", mask()),
            "SpriteMask { width: 3, height: 2, .. }"
        );
    }

    #[test]
    fn a_mask_from_an_image_is_its_alpha() {
        let mut pixels = vec![0; 3 * 2 * 4];
        pixels[4 + 3] = 1;
        pixels[5 * 4 + 3] = 255;
        // A colour with no alpha is clear.
        pixels[2 * 4] = 255;
        let image = Image::from_rgba(3, 2, pixels).expect("3 x 2");
        assert_eq!(SpriteMask::from_alpha(&image), mask());
    }

    #[test]
    fn its_opaque_pixel_is_the_centre_or_the_first_in_row_order() {
        assert_eq!(mask().opaque_pixel(), Some((1, 0)));
        let centred = SpriteMask::new(3, 3, vec![true; 9]).expect("9 pixels");
        assert_eq!(centred.opaque_pixel(), Some((1, 1)));
        let tall = SpriteMask::new(3, 5, vec![true; 15]).expect("15 pixels");
        assert_eq!(tall.opaque_pixel(), Some((1, 2)));
        let wide = SpriteMask::new(5, 3, vec![true; 15]).expect("15 pixels");
        assert_eq!(wide.opaque_pixel(), Some((2, 1)));
        let late =
            SpriteMask::new(2, 3, vec![false, false, false, false, false, true]).expect("6 pixels");
        assert_eq!(late.opaque_pixel(), Some((1, 2)));
        let clear = SpriteMask::new(2, 2, vec![false; 4]).expect("4 pixels");
        assert_eq!(clear.opaque_pixel(), None);
    }

    #[test]
    fn a_logo_frame_lies_along_its_row_then_down() {
        let logo = PictFrames {
            picture: 8010,
            frame_size: (10.0, 4.0),
            columns: 2,
            frames: 5,
            position: Point::new(0.0, 0.0),
        };
        let origins: Vec<Point> = (0..5).map(|frame| logo.frame_origin(frame)).collect();
        assert_eq!(
            origins,
            [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(0.0, 4.0),
                Point::new(10.0, 4.0),
                Point::new(0.0, 8.0),
            ]
        );
    }

    #[test]
    fn a_borrowed_or_shared_source_is_the_source() {
        struct Missing;
        impl MainScreenSource for Missing {
            fn main_screen_art(&self) -> Result<MainScreenArt, String> {
                Err("no cölr 128".to_owned())
            }
        }
        fn art_of(source: impl MainScreenSource) -> Result<MainScreenArt, String> {
            source.main_screen_art()
        }
        assert_eq!(art_of(&Missing), Err("no cölr 128".to_owned()));
        assert_eq!(art_of(Rc::new(Missing)), Err("no cölr 128".to_owned()));
    }
}
