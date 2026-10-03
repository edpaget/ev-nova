//! Texture atlas pages and the shelf packer that places images on them.
//!
//! Pages are square, [`PAGE_SIZE`] texels by default. Each page is packed
//! by a [`ShelfPacker`]: images sit left to right on horizontal shelves,
//! each new image going on the shortest shelf it fits (best fit) or on a new
//! shelf below the others. A [`GUTTER`] of transparent texels separates
//! neighbours, so nearest sampling at fractional scales never bleeds one
//! image into the next. Nothing is evicted.

use crate::viewport::PixelRect;

/// The default atlas page size: 2048x2048 RGBA8 texels (16 MiB), within
/// wgpu's downlevel texture limit and big enough for every stock `PICT`.
pub const PAGE_SIZE: u32 = 2048;

/// Transparent texels left to the right of and below each placement.
pub const GUTTER: u32 = 1;

/// Why an image could not be placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    /// The image is wider or taller than a whole page.
    #[error("a {width}x{height} image does not fit on a {page}x{page} atlas page")]
    TooLarge {
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// The page size.
        page: u32,
    },
    /// The page has no room left for the image.
    #[error("the atlas page is full")]
    PageFull,
}

/// One shelf: a row of images of at most `height` texels.
#[derive(Clone, Copy, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    cursor: u32,
}

/// Places rectangles on one square page by best-fit shelves. Pure
/// geometry: it holds no pixels.
#[derive(Clone, Debug)]
pub struct ShelfPacker {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
}

impl ShelfPacker {
    /// An empty `size` x `size` page.
    #[must_use]
    pub fn new(size: u32) -> Self {
        Self {
            size,
            shelves: Vec::new(),
            next_y: 0,
        }
    }

    /// The page's side length.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Places a `w` x `h` image: on the shortest shelf with room for it
    /// (the first of equals), else on a new shelf. Nothing changes on an
    /// error.
    pub fn place(&mut self, w: u32, h: u32) -> Result<PixelRect, PackError> {
        let size = self.size;
        if w > size || h > size {
            return Err(PackError::TooLarge {
                width: w,
                height: h,
                page: size,
            });
        }
        let best = self
            .shelves
            .iter_mut()
            .filter(|shelf| h <= shelf.height && shelf.cursor + w <= size)
            .min_by_key(|shelf| shelf.height);
        let target = match best {
            Some(fit) => fit,
            None if self.next_y + h <= size => {
                self.shelves.push(Shelf {
                    y: self.next_y,
                    height: h,
                    cursor: 0,
                });
                self.next_y += h + GUTTER;
                self.shelves.last_mut().expect("just pushed")
            }
            None => return Err(PackError::PageFull),
        };
        let at = PixelRect {
            x: target.cursor,
            y: target.y,
            w,
            h,
        };
        target.cursor += w + GUTTER;
        Ok(at)
    }
}

/// An atlas page, numbered from 0 in the order pages open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId(pub u32);

/// Texture coordinates of an image on its page, 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Uv {
    /// Left.
    pub u0: f32,
    /// Top.
    pub v0: f32,
    /// Right.
    pub u1: f32,
    /// Bottom.
    pub v1: f32,
}

/// Where an image was placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasEntry {
    /// The page.
    pub page: PageId,
    /// The texels it covers.
    pub rect: PixelRect,
    /// The same, as texture coordinates.
    pub uv: Uv,
    /// Whether this placement opened `page`.
    pub opened: bool,
}

/// Atlas pages of one size. Images go on the newest page; when it is full,
/// a new page opens.
#[derive(Clone, Debug)]
pub struct Atlas {
    page_size: u32,
    pages: Vec<ShelfPacker>,
}

impl Atlas {
    /// An atlas with no pages yet, whose pages will be `page_size` square.
    #[must_use]
    pub fn new(page_size: u32) -> Self {
        Self {
            page_size,
            pages: Vec::new(),
        }
    }

    /// The pages' side length.
    #[must_use]
    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    /// How many pages are open.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Places a `w` x `h` image on the newest page, opening a page when
    /// there is none or it is full. An image bigger than a page opens
    /// nothing.
    pub fn place(&mut self, w: u32, h: u32) -> Result<AtlasEntry, PackError> {
        if let Some(page) = self.pages.last_mut() {
            match page.place(w, h) {
                Ok(rect) => return Ok(self.entry(self.pages.len() - 1, rect, false)),
                Err(PackError::PageFull) => {}
                Err(too_large) => return Err(too_large),
            }
        }
        let mut page = ShelfPacker::new(self.page_size);
        let rect = page.place(w, h)?;
        self.pages.push(page);
        Ok(self.entry(self.pages.len() - 1, rect, true))
    }

    fn entry(&self, page: usize, rect: PixelRect, opened: bool) -> AtlasEntry {
        let size = self.page_size as f32;
        AtlasEntry {
            page: PageId(page as u32),
            rect,
            uv: Uv {
                u0: rect.x as f32 / size,
                v0: rect.y as f32 / size,
                u1: (rect.x + rect.w) as f32 / size,
                v1: (rect.y + rect.h) as f32 / size,
            },
            opened,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    fn place(packer: &mut ShelfPacker, w: u32, h: u32) -> PixelRect {
        packer
            .place(w, h)
            .unwrap_or_else(|e| panic!("{w}x{h}: {e}"))
    }

    #[test]
    fn the_default_page_is_2048_square_with_a_one_texel_gutter() {
        assert_eq!(PAGE_SIZE, 2048);
        assert_eq!(GUTTER, 1);
    }

    #[test]
    fn images_fill_shelves_left_to_right_then_open_new_shelves() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(packer.size(), 64);
        assert_eq!(place(&mut packer, 32, 16), rect(0, 0, 32, 16));
        // Exactly fills the rest of the shelf: touching the right edge.
        assert_eq!(place(&mut packer, 31, 16), rect(33, 0, 31, 16));
        assert_eq!(place(&mut packer, 16, 32), rect(0, 17, 16, 32));
        assert_eq!(place(&mut packer, 8, 8), rect(17, 17, 8, 8));
    }

    #[test]
    fn an_image_goes_on_the_shortest_shelf_it_fits() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(place(&mut packer, 60, 16), rect(0, 0, 60, 16));
        assert_eq!(place(&mut packer, 10, 8), rect(0, 17, 10, 8));
        // Fits both shelves; the 8-high one is the better fit.
        assert_eq!(place(&mut packer, 3, 8), rect(11, 17, 3, 8));
        // Too tall for the 8-high shelf.
        assert_eq!(place(&mut packer, 3, 9), rect(61, 0, 3, 9));
    }

    #[test]
    fn equally_good_shelves_go_to_the_first() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(place(&mut packer, 50, 8), rect(0, 0, 50, 8));
        assert_eq!(place(&mut packer, 20, 8), rect(0, 9, 20, 8));
        // Both 8-high shelves have room; the first wins.
        assert_eq!(place(&mut packer, 10, 8), rect(51, 0, 10, 8));
    }

    #[test]
    fn a_shelf_without_room_is_skipped() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(place(&mut packer, 60, 8), rect(0, 0, 60, 8));
        assert_eq!(place(&mut packer, 10, 16), rect(0, 9, 10, 16));
        // Fits the 8-high shelf by height but not by width.
        assert_eq!(place(&mut packer, 4, 8), rect(11, 9, 4, 8));
    }

    #[test]
    fn a_new_shelf_may_touch_the_bottom_edge() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(place(&mut packer, 64, 31), rect(0, 0, 64, 31));
        assert_eq!(place(&mut packer, 64, 32), rect(0, 32, 64, 32));
        assert_eq!(packer.place(1, 1), Err(PackError::PageFull));
    }

    #[test]
    fn a_page_sized_image_fits_exactly_and_fills_the_page() {
        let mut packer = ShelfPacker::new(64);
        assert_eq!(place(&mut packer, 64, 64), rect(0, 0, 64, 64));
        assert_eq!(packer.place(1, 1), Err(PackError::PageFull));
    }

    #[test]
    fn an_image_bigger_than_a_page_is_rejected_without_changing_anything() {
        let mut packer = ShelfPacker::new(64);
        let too_wide = PackError::TooLarge {
            width: 65,
            height: 10,
            page: 64,
        };
        assert_eq!(packer.place(65, 10), Err(too_wide));
        assert_eq!(
            packer.place(10, 65),
            Err(PackError::TooLarge {
                width: 10,
                height: 65,
                page: 64
            })
        );
        assert_eq!(place(&mut packer, 4, 4), rect(0, 0, 4, 4));
        assert_eq!(
            too_wide.to_string(),
            "a 65x10 image does not fit on a 64x64 atlas page"
        );
    }

    /// Whether `a` and `b` overlap once each is grown by the gutter.
    fn touch(a: PixelRect, b: PixelRect) -> bool {
        a.x < b.x + b.w + GUTTER
            && b.x < a.x + a.w + GUTTER
            && a.y < b.y + b.h + GUTTER
            && b.y < a.y + a.h + GUTTER
    }

    #[test]
    fn placements_never_overlap_and_stay_on_the_page() {
        let sizes = [
            (12, 7),
            (3, 30),
            (20, 7),
            (5, 5),
            (40, 2),
            (7, 12),
            (1, 1),
            (9, 6),
            (16, 16),
            (2, 9),
            (11, 3),
            (6, 6),
            (30, 4),
            (4, 4),
            (8, 2),
        ];
        let mut packer = ShelfPacker::new(64);
        let mut placed: Vec<PixelRect> = Vec::new();
        for (w, h) in sizes {
            let at = place(&mut packer, w, h);
            assert_eq!((at.w, at.h), (w, h));
            assert!(
                at.x + at.w <= 64 && at.y + at.h <= 64,
                "{at:?} off the page"
            );
            for &other in &placed {
                assert!(!touch(at, other), "{at:?} touches {other:?}");
            }
            placed.push(at);
        }
        assert_eq!(placed.len(), sizes.len());
    }

    #[test]
    fn uvs_are_the_rect_over_the_page_size() {
        let mut atlas = Atlas::new(64);
        assert_eq!(atlas.page_size(), 64);
        atlas.place(32, 16).unwrap();
        let entry = atlas.place(31, 16).unwrap();
        assert_eq!(entry.rect, rect(33, 0, 31, 16));
        assert_eq!(
            entry.uv,
            Uv {
                u0: 33.0 / 64.0,
                v0: 0.0,
                u1: 1.0,
                v1: 0.25
            }
        );
        let lower = atlas.place(16, 32).unwrap();
        assert_eq!(
            lower.uv,
            Uv {
                u0: 0.0,
                v0: 17.0 / 64.0,
                u1: 0.25,
                v1: 49.0 / 64.0
            }
        );
    }

    #[test]
    fn the_first_placement_opens_page_zero() {
        let mut atlas = Atlas::new(64);
        assert_eq!(atlas.page_count(), 0);
        let entry = atlas.place(8, 8).unwrap();
        assert_eq!(entry.page, PageId(0));
        assert!(entry.opened);
        assert_eq!(atlas.page_count(), 1);
        let next = atlas.place(8, 8).unwrap();
        assert_eq!((next.page, next.opened), (PageId(0), false));
    }

    #[test]
    fn a_full_page_opens_the_next_one() {
        let mut atlas = Atlas::new(64);
        atlas.place(64, 40).unwrap();
        let entry = atlas.place(64, 40).unwrap();
        assert_eq!(entry.page, PageId(1));
        assert!(entry.opened);
        assert_eq!(entry.rect, rect(0, 0, 64, 40));
        assert_eq!(atlas.page_count(), 2);
        // Later images go on the newest page.
        let small = atlas.place(8, 8).unwrap();
        assert_eq!((small.page, small.opened), (PageId(1), false));
        assert_eq!(small.rect, rect(0, 41, 8, 8));
    }

    #[test]
    fn an_oversize_image_opens_no_page() {
        let mut atlas = Atlas::new(64);
        assert_eq!(
            atlas.place(65, 1).unwrap_err(),
            PackError::TooLarge {
                width: 65,
                height: 1,
                page: 64
            }
        );
        assert_eq!(atlas.page_count(), 0);
        atlas.place(64, 64).unwrap();
        assert!(matches!(
            atlas.place(1, 65),
            Err(PackError::TooLarge { .. })
        ));
        assert_eq!(atlas.page_count(), 1);
    }
}
