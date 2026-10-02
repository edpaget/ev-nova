//! QuickDraw rectangles and `PixMap` headers.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4, "Color
//! QuickDraw": the `PixMap` record. Rectangles are `top, left, bottom,
//! right` as signed 16-bit coordinates.

use super::GraphicsError;
use super::reader::Reader;

/// `rowBytes` bit 15: the record is a `PixMap`, not a `BitMap`.
const PIXMAP_FLAG: u16 = 0x8000;
/// The bits of `rowBytes` that hold the count (bits 14 and 15 are flags).
const ROW_BYTES_MASK: u16 = 0x3FFF;

/// A QuickDraw rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QdRect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

impl QdRect {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, GraphicsError> {
        Ok(Self {
            top: r.i16()?,
            left: r.i16()?,
            bottom: r.i16()?,
            right: r.i16()?,
        })
    }

    /// `right - left`, computed without 16-bit overflow.
    pub(crate) fn width(self) -> i32 {
        i32::from(self.right) - i32::from(self.left)
    }

    /// `bottom - top`, computed without 16-bit overflow.
    pub(crate) fn height(self) -> i32 {
        i32::from(self.bottom) - i32::from(self.top)
    }

    /// The width and height, or [`GraphicsError::BadDimensions`] unless both
    /// are positive.
    pub(crate) fn size(self) -> Result<(u32, u32), GraphicsError> {
        let (width, height) = (self.width(), self.height());
        if width <= 0 || height <= 0 {
            return Err(GraphicsError::BadDimensions { width, height });
        }
        Ok((width as u32, height as u32))
    }

    /// Whether `inner` lies entirely inside this rectangle.
    pub(crate) fn contains(self, inner: Self) -> bool {
        inner.top >= self.top
            && inner.left >= self.left
            && inner.bottom <= self.bottom
            && inner.right <= self.right
    }
}

/// The fields of a `PixMap` record the decoders use. `baseAddr`, which
/// precedes it in some formats, is skipped by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixMap {
    /// `rowBytes` with the flag bits masked off.
    pub row_bytes: u16,
    pub bounds: QdRect,
    pub pack_type: u16,
    pub pixel_size: u16,
    pub cmp_count: u16,
    pub cmp_size: u16,
    /// Offset of the colour table, where the format uses it (`ppat`).
    pub pm_table: u32,
}

impl PixMap {
    /// Reads the 46 bytes from `rowBytes` to `pmReserved`. A `rowBytes`
    /// without bit 15 set marks a plain `BitMap`, which is
    /// [`GraphicsError::NotAPixMap`].
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, GraphicsError> {
        let offset = r.pos();
        let row_bytes = r.u16()?;
        if row_bytes & PIXMAP_FLAG == 0 {
            return Err(GraphicsError::NotAPixMap { offset });
        }
        let bounds = QdRect::read(r)?;
        r.skip(2)?; // pmVersion
        let pack_type = r.u16()?;
        r.skip(14)?; // packSize, hRes, vRes, pixelType
        let pixel_size = r.u16()?;
        let cmp_count = r.u16()?;
        let cmp_size = r.u16()?;
        r.skip(4)?; // planeBytes
        let pm_table = r.u32()?;
        r.skip(4)?; // pmReserved
        Ok(Self {
            row_bytes: row_bytes & ROW_BYTES_MASK,
            bounds,
            pack_type,
            pixel_size,
            cmp_count,
            cmp_size,
            pm_table,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: [u8; 8] = [0xFF, 0xFE, 0x00, 0x03, 0x00, 0x05, 0x00, 0x0A];

    #[test]
    fn rectangles_read_top_left_bottom_right() {
        let mut r = Reader::new(&RECT);
        let rect = QdRect::read(&mut r).unwrap();
        assert_eq!(
            rect,
            QdRect {
                top: -2,
                left: 3,
                bottom: 5,
                right: 10
            }
        );
        assert_eq!(r.pos(), 8);
        assert_eq!((rect.width(), rect.height()), (7, 7));
        assert_eq!(
            QdRect::read(&mut Reader::new(&RECT[..7])),
            Err(GraphicsError::UnexpectedEnd { offset: 6 })
        );
    }

    fn rect(top: i16, left: i16, bottom: i16, right: i16) -> QdRect {
        QdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    #[test]
    fn sizes_do_not_overflow_16_bits() {
        let r = rect(-32768, -32768, 32767, 32767);
        assert_eq!((r.width(), r.height()), (65535, 65535));
        assert_eq!(r.size(), Ok((65535, 65535)));
        assert_eq!(rect(1, 2, 4, 7).size(), Ok((5, 3)));
    }

    #[test]
    fn empty_or_inverted_rectangles_have_bad_dimensions() {
        let bad = |width, height| Err(GraphicsError::BadDimensions { width, height });
        assert_eq!(rect(0, 0, 0, 5).size(), bad(5, 0));
        assert_eq!(rect(0, 0, 5, 0).size(), bad(0, 5));
        assert_eq!(rect(5, 0, 0, 1).size(), bad(1, -5));
        assert_eq!(rect(0, 5, 1, 0).size(), bad(-5, 1));
    }

    #[test]
    fn contains_includes_the_edges() {
        let outer = rect(0, 0, 10, 20);
        assert!(outer.contains(outer));
        assert!(outer.contains(rect(1, 1, 9, 19)));
        assert!(!outer.contains(rect(-1, 0, 10, 20)));
        assert!(!outer.contains(rect(0, -1, 10, 20)));
        assert!(!outer.contains(rect(0, 0, 11, 20)));
        assert!(!outer.contains(rect(0, 0, 10, 21)));
    }

    /// A 16-bit `PixMap`, written out by hand.
    const PIXMAP: [u8; 46] = [
        0x80, 0x0C, // rowBytes 12, PixMap flag
        0, 1, 0, 2, 0, 4, 0, 8, // bounds (1, 2) to (4, 8)
        0, 0, // pmVersion
        0, 3, // packType
        0, 0, 0, 0, // packSize
        0, 0x48, 0, 0, 0, 0x48, 0, 0, // hRes, vRes
        0, 0x10, // pixelType
        0, 16, // pixelSize
        0, 3, // cmpCount
        0, 5, // cmpSize
        0, 0, 0, 0, // planeBytes
        0x12, 0x34, 0x56, 0x78, // pmTable
        0, 0, 0, 0, // pmReserved
    ];

    #[test]
    fn pixmap_fields_are_read_from_their_offsets() {
        let mut r = Reader::new(&PIXMAP);
        let pm = PixMap::read(&mut r).unwrap();
        assert_eq!(
            pm,
            PixMap {
                row_bytes: 12,
                bounds: rect(1, 2, 4, 8),
                pack_type: 3,
                pixel_size: 16,
                cmp_count: 3,
                cmp_size: 5,
                pm_table: 0x1234_5678,
            }
        );
        assert_eq!(r.pos(), 46);
    }

    #[test]
    fn pixmap_row_bytes_drop_both_flag_bits() {
        let mut bytes = PIXMAP;
        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        let pm = PixMap::read(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(pm.row_bytes, 0x3FFF);
    }

    #[test]
    fn a_bitmap_is_not_a_pixmap() {
        let mut bytes = [0xEE; 48];
        bytes[2..].copy_from_slice(&PIXMAP);
        bytes[2] = 0x7F;
        let mut r = Reader::new(&bytes);
        r.skip(2).unwrap();
        assert_eq!(
            PixMap::read(&mut r),
            Err(GraphicsError::NotAPixMap { offset: 2 })
        );
    }

    #[test]
    fn a_truncated_pixmap_is_an_unexpected_end() {
        assert_eq!(
            PixMap::read(&mut Reader::new(&PIXMAP[..45])),
            Err(GraphicsError::UnexpectedEnd { offset: 42 })
        );
        assert_eq!(
            PixMap::read(&mut Reader::new(&PIXMAP[..1])),
            Err(GraphicsError::UnexpectedEnd { offset: 0 })
        );
    }
}
