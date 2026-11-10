//! Indexed `PixMap`s: the supported-depth check, size and `rowBytes`
//! validation, and decoding through a colour table. `cicn`, `ppat` and the
//! indexed `PICT` bits opcodes share them.

use super::GraphicsError;
use super::budget::check_budget;
use super::color::{ColorTable, indices};
use super::image::Image;
use super::pixmap::PixMap;

/// The one depth check that `cicn`, `ppat` and the indexed `PICT` bits
/// opcodes share: a `pixel_size` of 1, 2, 4 or 8, or
/// [`GraphicsError::UnsupportedPixMap`] naming the map.
pub(super) fn check_indexed_depth(pm: &PixMap) -> Result<(), GraphicsError> {
    if !matches!(pm.pixel_size, 1 | 2 | 4 | 8) {
        return Err(GraphicsError::UnsupportedPixMap {
            pixel_size: pm.pixel_size,
            pack_type: pm.pack_type,
            cmp_count: pm.cmp_count,
        });
    }
    Ok(())
}

/// The size of an indexed (1, 2, 4 or 8-bit) `PixMap`, checked against the
/// budget for `input_len` bytes and against its `rowBytes`.
pub(super) fn checked_indexed_size(
    pm: &PixMap,
    input_len: usize,
) -> Result<(u32, u32), GraphicsError> {
    let (width, height) = pm.bounds.size()?;
    check_budget(u64::from(width) * u64::from(height), input_len)?;
    check_indexed_depth(pm)?;
    check_row_bytes(pm.row_bytes, (width * u32::from(pm.pixel_size)).div_ceil(8))?;
    Ok((width, height))
}

/// [`GraphicsError::BadRowBytes`] unless `row_bytes` holds `needed` bytes.
pub(super) fn check_row_bytes(row_bytes: u16, needed: u32) -> Result<(), GraphicsError> {
    if u32::from(row_bytes) < needed {
        return Err(GraphicsError::BadRowBytes { row_bytes, needed });
    }
    Ok(())
}

/// Colours unpacked indexed `pixels` rows (`rowBytes` each) through `ctab`.
/// Each row of `mask` (one bit per pixel, 1 opaque) masks a pixel row;
/// pixels without a mask row are opaque.
pub(super) fn indexed_image<'m>(
    pixels: &[u8],
    pm: &PixMap,
    ctab: &ColorTable,
    width: u32,
    height: u32,
    mut mask: impl Iterator<Item = &'m [u8]>,
) -> Result<Image, GraphicsError> {
    let mut image = Image::transparent(width, height);
    let rows = pixels.chunks(usize::from(pm.row_bytes));
    for (y, row) in (0..height).zip(rows) {
        let values = indices(row, pm.pixel_size, width as usize).unwrap_or_default();
        let opaque = match mask.next() {
            Some(bits) => indices(bits, 1, width as usize).unwrap_or_default(),
            None => vec![1; width as usize],
        };
        for ((x, value), opaque) in (0..width).zip(values).zip(opaque) {
            if opaque == 1 {
                image.put(x, y, ctab.rgba(value)?);
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::pixmap::QdRect;

    fn pixmap(pixel_size: u16) -> PixMap {
        let bounds = QdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        PixMap {
            pixel_size,
            pack_type: 4,
            cmp_count: 3,
            ..PixMap::bitmap(1, bounds)
        }
    }

    #[test]
    fn every_indexed_depth_is_supported() {
        for depth in [1, 2, 4, 8] {
            assert_eq!(check_indexed_depth(&pixmap(depth)), Ok(()), "{depth}");
        }
    }

    #[test]
    fn other_depths_are_unsupported_with_the_pixmap_named() {
        for depth in [0, 3, 16, 32] {
            assert_eq!(
                check_indexed_depth(&pixmap(depth)),
                Err(GraphicsError::UnsupportedPixMap {
                    pixel_size: depth,
                    pack_type: 4,
                    cmp_count: 3,
                }),
            );
        }
    }
}
