//! Sprite sheets: decoded frames and how to arrange them.

use std::num::NonZeroU16;

use super::image::Image;

/// How a sheet's frames are arranged when composed into one image: row
/// by row, left to right, `columns` frames per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SheetLayout {
    columns: NonZeroU16,
}

impl SheetLayout {
    /// The layout for sheets with no `spïn` or `shän` to say otherwise: 6
    /// columns. The value is arbitrary but fixed; it matches the classic
    /// 6-wide grid of a 36-frame rotation.
    pub const DEFAULT: Self = Self {
        columns: NonZeroU16::new(6).expect("6 is not zero"),
    };

    /// A layout of `columns` frames per row, or `None` for 0.
    #[must_use]
    pub fn new(columns: u16) -> Option<Self> {
        NonZeroU16::new(columns).map(|columns| Self { columns })
    }

    /// Frames per row.
    #[must_use]
    pub fn columns(self) -> u16 {
        self.columns.get()
    }
}

/// A decoded sprite sheet: equal-sized frames in resource order, plus the
/// layout to arrange them in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteSheet {
    frame_width: u32,
    frame_height: u32,
    frames: Vec<Image>,
    layout: SheetLayout,
}

impl SpriteSheet {
    /// Callers guarantee at least one frame, each `frame_width` x
    /// `frame_height`.
    pub(crate) fn new(
        frame_width: u32,
        frame_height: u32,
        frames: Vec<Image>,
        layout: SheetLayout,
    ) -> Self {
        Self {
            frame_width,
            frame_height,
            frames,
            layout,
        }
    }

    /// Width of every frame.
    #[must_use]
    pub fn frame_width(&self) -> u32 {
        self.frame_width
    }

    /// Height of every frame.
    #[must_use]
    pub fn frame_height(&self) -> u32 {
        self.frame_height
    }

    /// The frames, in resource order.
    #[must_use]
    pub fn frames(&self) -> &[Image] {
        &self.frames
    }

    /// The frames, by value.
    #[must_use]
    pub fn into_frames(self) -> Vec<Image> {
        self.frames
    }

    /// The layout the sheet was decoded with.
    #[must_use]
    pub fn layout(&self) -> SheetLayout {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_have_at_least_one_column() {
        assert_eq!(SheetLayout::new(0), None);
        assert_eq!(SheetLayout::new(1).map(SheetLayout::columns), Some(1));
        assert_eq!(SheetLayout::new(36).map(SheetLayout::columns), Some(36));
        assert_eq!(SheetLayout::DEFAULT.columns(), 6);
    }

    #[test]
    fn accessors_return_what_the_sheet_was_built_with() {
        let frames = vec![
            Image::from_rgba(2, 1, vec![1; 8]).unwrap(),
            Image::from_rgba(2, 1, vec![2; 8]).unwrap(),
        ];
        let layout = SheetLayout::new(3).unwrap();
        let sheet = SpriteSheet::new(2, 1, frames.clone(), layout);
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (2, 1));
        assert_eq!(sheet.frames(), frames);
        assert_eq!(sheet.layout(), layout);
        assert_eq!(sheet.into_frames(), frames);
    }
}
