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
///
/// Each frame has a box, the header's width x height: what is clicked, and
/// what touches and placement are measured by. Its image is usually the
/// box, but a sheet the original draws past its box (an `rlëD` of depth 32,
/// say; see `rled_wide`) has larger images, with the box at
/// [`box_origin`](Self::box_origin) and the image padded so the box sits
/// at its centre: a view that centres the image on the box's centre draws
/// it where the original does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteSheet {
    frame_width: u32,
    frame_height: u32,
    image_width: u32,
    image_height: u32,
    box_origin: (u32, u32),
    frames: Vec<Image>,
    layout: SheetLayout,
    /// The frames' solid runs alone, when any frame has one.
    solid: Option<Vec<Image>>,
    /// The depth the original's draw proc sees (see [`Self::depth`]).
    depth: u16,
    /// Whether its clipped draw can differ from a crop (see
    /// [`Self::clip_quirks`]).
    clip_quirks: bool,
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
            image_width: frame_width,
            image_height: frame_height,
            box_origin: (0, 0),
            frames,
            layout,
            solid: None,
            depth: 16,
            clip_quirks: false,
        }
    }

    /// A sheet whose frames' boxes are `frame_size` but whose images are
    /// `image_size`, each box at `box_origin` in its image. Callers
    /// guarantee at least one frame, each of `image_size`, and a box that
    /// fits inside it.
    pub(crate) fn overhanging(
        frame_size: (u32, u32),
        image_size: (u32, u32),
        box_origin: (u32, u32),
        frames: Vec<Image>,
        layout: SheetLayout,
    ) -> Self {
        Self {
            image_width: image_size.0,
            image_height: image_size.1,
            box_origin,
            ..Self::new(frame_size.0, frame_size.1, frames, layout)
        }
    }

    /// The sheet with `solid` as its solid-run frames: one per frame, or
    /// none.
    pub(crate) fn with_solid(self, solid: Option<Vec<Image>>) -> Self {
        Self { solid, ..self }
    }

    /// The sheet with `depth` as its header's depth (16 unless set).
    pub(crate) fn with_depth(self, depth: u16) -> Self {
        Self { depth, ..self }
    }

    /// The sheet with `clip_quirks` as [`Self::clip_quirks`] (false unless
    /// set).
    pub(crate) fn with_clip_quirks(self, clip_quirks: bool) -> Self {
        Self {
            clip_quirks,
            ..self
        }
    }

    /// Whether a frame whose box touches the world's edge may draw
    /// otherwise than its image cut to the box, as `_BlitPixieRLEClipped`
    /// draws it ([`decode_rled_clipped`](super::decode_rled_clipped)): a
    /// line start whose count lies, a run whose fill shows a restart at
    /// the cut, or a token ending on an even byte. A conservative test,
    /// false for every stock sheet, for a `PICT` pair's cut, and for a
    /// sheet read otherwise than the original draws it.
    #[must_use]
    pub fn clip_quirks(&self) -> bool {
        self.clip_quirks
    }

    /// Width of every frame's box: the header's width.
    #[must_use]
    pub fn frame_width(&self) -> u32 {
        self.frame_width
    }

    /// Height of every frame's box: the header's height.
    #[must_use]
    pub fn frame_height(&self) -> u32 {
        self.frame_height
    }

    /// Width of every frame's image: its box's, unless the sheet draws
    /// past its box.
    #[must_use]
    pub fn image_width(&self) -> u32 {
        self.image_width
    }

    /// Height of every frame's image: its box's, unless the sheet draws
    /// past its box.
    #[must_use]
    pub fn image_height(&self) -> u32 {
        self.image_height
    }

    /// Where each frame's box's top-left corner lies in its image: (0, 0)
    /// unless the sheet draws past its box.
    #[must_use]
    pub fn box_origin(&self) -> (u32, u32) {
        self.box_origin
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

    /// Its solid runs alone, one image per frame, when any frame has a
    /// run of one colour: an `rlëD`'s opcode-4 runs, which the original
    /// draws raw over every blend and colorize (see
    /// [`decode_rled`](super::rled::decode_rled)). None for a sheet with
    /// none.
    #[must_use]
    pub fn solid_frames(&self) -> Option<&[Image]> {
        self.solid.as_deref()
    }

    /// Its solid-run frames ([`SpriteSheet::solid_frames`]), by value.
    #[must_use]
    pub fn into_solid_frames(self) -> Option<Vec<Image>> {
        self.solid
    }

    /// The depth of the frames the original's draw proc sees, which it
    /// branches on (+0x20, @0xbb5a4, @0xbb792): an `rlëD`'s header depth
    /// (16, 8, 32 or another), under either reading, and 16 for a `PICT`
    /// pair's cut, which `_ConvertSpriteToRLE` makes a 16-bit RLE.
    #[must_use]
    pub fn depth(&self) -> u16 {
        self.depth
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

    /// One image of every frame, placed row by row, left to right, a cell
    /// the size of a frame's image; trailing cells of the last row stay
    /// transparent.
    ///
    /// Clamping the columns to the frame count keeps the composed image
    /// under twice the frames' own pixels, which the decoder has already
    /// checked against its budget.
    #[must_use]
    pub fn compose(&self) -> Image {
        let columns = self.columns();
        let mut sheet =
            Image::transparent(columns * self.image_width, self.rows() * self.image_height);
        for (i, frame) in (0..).zip(&self.frames) {
            let x = i % columns * self.image_width;
            let y = i / columns * self.image_height;
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
    fn a_plain_sheets_images_are_its_box() {
        let plain = sheet(2, 2);
        assert_eq!((plain.image_width(), plain.image_height()), (2, 1));
        assert_eq!(plain.box_origin(), (0, 0));
    }

    /// Two 6x1 images, each filled with its own index plus one, around a
    /// 2x1 box at (2, 0).
    fn overhanging(columns: u16) -> SpriteSheet {
        let frames = (1..=2)
            .map(|i| Image::from_rgba(6, 1, vec![i; 24]).unwrap())
            .collect();
        SpriteSheet::overhanging(
            (2, 1),
            (6, 1),
            (2, 0),
            frames,
            SheetLayout::new(columns).unwrap(),
        )
    }

    #[test]
    fn a_sheet_keeps_its_box_while_its_images_are_wider() {
        let wide = overhanging(2);
        assert_eq!((wide.frame_width(), wide.frame_height()), (2, 1));
        assert_eq!((wide.image_width(), wide.image_height()), (6, 1));
        assert_eq!(wide.box_origin(), (2, 0));
        assert_eq!(wide.frames().len(), 2);
    }

    #[test]
    fn compose_lays_cells_out_by_the_image_size() {
        let composed = overhanging(2).compose();
        assert_eq!((composed.width(), composed.height()), (12, 1));
        assert_eq!(cells(&composed), [1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2]);
        let composed = overhanging(1).compose();
        assert_eq!((composed.width(), composed.height()), (6, 2));
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
