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

    /// Frames per row when composed: the layout's columns, but never more
    /// than there are frames.
    #[must_use]
    pub fn columns(&self) -> u32 {
        u32::from(self.layout.columns()).min(self.frames.len() as u32)
    }

    /// Rows when composed: enough for every frame.
    #[must_use]
    pub fn rows(&self) -> u32 {
        (self.frames.len() as u32).div_ceil(self.columns())
    }

    /// One image of every frame, placed row by row, left to right;
    /// trailing cells of the last row stay transparent.
    ///
    /// Clamping the columns to the frame count keeps the composed image
    /// under twice the frames' own pixels, which the decoder has already
    /// checked against its budget.
    #[must_use]
    pub fn compose(&self) -> Image {
        let columns = self.columns();
        let mut sheet =
            Image::transparent(columns * self.frame_width, self.rows() * self.frame_height);
        for (i, frame) in (0..).zip(&self.frames) {
            let x = i % columns * self.frame_width;
            let y = i / columns * self.frame_height;
            sheet.blit(frame, x, y);
        }
        sheet
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

    /// Frames of 2x1 pixels, each filled with its own index plus one.
    fn sheet(count: u8, columns: u16) -> SpriteSheet {
        let frames = (1..=count)
            .map(|i| Image::from_rgba(2, 1, vec![i; 8]).unwrap())
            .collect();
        SpriteSheet::new(2, 1, frames, SheetLayout::new(columns).unwrap())
    }

    /// The composed image as one value per pixel.
    fn cells(image: &Image) -> Vec<u8> {
        image.pixels().chunks(4).map(|p| p[0]).collect()
    }

    #[test]
    fn columns_and_rows_cover_every_frame() {
        let grid = |s: SpriteSheet| (s.columns(), s.rows());
        assert_eq!(grid(sheet(6, 2)), (2, 3));
        assert_eq!(grid(sheet(7, 2)), (2, 4));
        assert_eq!(grid(sheet(6, 6)), (6, 1));
        assert_eq!(grid(sheet(36, 6)), (6, 6));
        assert_eq!(grid(sheet(1, 1)), (1, 1));
    }

    #[test]
    fn columns_are_clamped_to_the_frame_count() {
        assert_eq!(sheet(4, 36).columns(), 4);
        assert_eq!(sheet(4, 36).rows(), 1);
        let composed = sheet(4, 36).compose();
        assert_eq!((composed.width(), composed.height()), (8, 1));
    }

    #[test]
    fn compose_places_frames_row_by_row() {
        let composed = sheet(5, 2).compose();
        assert_eq!((composed.width(), composed.height()), (4, 3));
        assert_eq!(cells(&composed), [1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0, 0]);
        let composed = sheet(5, 3).compose();
        assert_eq!((composed.width(), composed.height()), (6, 2));
        assert_eq!(cells(&composed), [1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 0, 0]);
    }

    #[test]
    fn compose_keeps_each_frame_s_rows_together() {
        let frames = vec![
            Image::from_rgba(1, 2, vec![1, 1, 1, 1, 2, 2, 2, 2]).unwrap(),
            Image::from_rgba(1, 2, vec![3, 3, 3, 3, 4, 4, 4, 4]).unwrap(),
        ];
        let tall = SpriteSheet::new(1, 2, frames, SheetLayout::DEFAULT);
        assert_eq!(cells(&tall.compose()), [1, 3, 2, 4]);
    }
}
