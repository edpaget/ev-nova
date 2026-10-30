//! QuickDraw regions: the mask a `PICT`'s `0091`, `0099` and `009B`
//! opcodes copy through.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 2, the
//! `Region` record (`rgnSize`, which counts itself, then `rgnBBox`); a
//! region of exactly 10 bytes is its bounding rectangle. The data after a
//! larger region's box is read as inversion points, the layout QuickDraw
//! stores but does not document: for each line, its `y`, then the `x`s at
//! which the line's membership flips relative to the line above it, then
//! `0x7FFF`; after the last line, a final `0x7FFF`. A point is inside when
//! an odd number of inversion points lie at or above it and at or to its
//! left. That is a reading of the format, not code copied from anywhere.

use std::collections::BTreeSet;

use super::GraphicsError;
use super::pixmap::QdRect;
use super::reader::Reader;

/// The smallest region: its size word and bounding rectangle.
pub(crate) const MIN_REGION: u16 = 10;
/// Ends a line's points, and the region.
const END: i16 = 0x7FFF;

/// A region, in its own (a copy's destination) coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    bbox: QdRect,
    /// For each inversion line, its `y` and the `x`s at which membership
    /// flips along it, accumulated over every line up to it, ascending. Empty
    /// for a rectangular region.
    lines: Vec<(i16, Vec<i16>)>,
}

impl Region {
    /// Reads a region. A malformed one is [`GraphicsError::BadRegion`] at
    /// the region's start: a size under 10, a size that runs past the
    /// data, a line with an odd number of points, a missing terminator, or
    /// lines or points out of order.
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, GraphicsError> {
        let offset = r.pos();
        let size = r.u16()?;
        let bad = GraphicsError::BadRegion { size, offset };
        if size < MIN_REGION {
            return Err(bad);
        }
        let body = r.bytes(usize::from(size - 2)).map_err(|_| bad.clone())?;
        let mut body = Reader::new(body);
        let bbox = QdRect::read(&mut body)?;
        let lines = if size == MIN_REGION {
            Vec::new()
        } else {
            read_lines(&mut body).ok_or(bad)?
        };
        Ok(Self { bbox, lines })
    }

    /// Whether the pixel at (`x`, `y`) is inside the region.
    pub(crate) fn contains(&self, x: i32, y: i32) -> bool {
        let in_box = x >= i32::from(self.bbox.left)
            && x < i32::from(self.bbox.right)
            && y >= i32::from(self.bbox.top)
            && y < i32::from(self.bbox.bottom);
        if !in_box || self.lines.is_empty() {
            return in_box;
        }
        let line = self.lines.partition_point(|(ly, _)| i32::from(*ly) <= y);
        let Some((_, flips)) = line.checked_sub(1).and_then(|i| self.lines.get(i)) else {
            return false;
        };
        flips.partition_point(|&fx| i32::from(fx) <= x) % 2 == 1
    }
}

/// The inversion lines after the bounding box, accumulated, or `None` if
/// they are malformed. Every loop reads at least one word, so the data
/// bounds them.
fn read_lines(body: &mut Reader<'_>) -> Option<Vec<(i16, Vec<i16>)>> {
    let mut lines: Vec<(i16, Vec<i16>)> = Vec::new();
    let mut flips = BTreeSet::new();
    loop {
        let y = body.i16().ok()?;
        if y == END {
            return Some(lines);
        }
        if lines.last().is_some_and(|&(last, _)| y <= last) {
            return None;
        }
        let mut points = 0usize;
        let mut last_x = None;
        loop {
            let x = body.i16().ok()?;
            if x == END {
                break;
            }
            if last_x.is_some_and(|last| x <= last) {
                return None;
            }
            last_x = Some(x);
            points += 1;
            if !flips.remove(&x) {
                flips.insert(x);
            }
        }
        if points % 2 == 1 {
            return None;
        }
        lines.push((y, flips.iter().copied().collect()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::{inversion_region, rect_region};

    fn read(bytes: &[u8]) -> Result<Region, GraphicsError> {
        Region::read(&mut Reader::new(bytes))
    }

    /// The points of `[top, left, bottom, right]` inside `region`, as rows
    /// of `#` and `.`.
    fn art(region: &Region, area: [i32; 4]) -> Vec<String> {
        (area[0]..area[2])
            .map(|y| {
                (area[1]..area[3])
                    .map(|x| if region.contains(x, y) { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_ten_byte_region_is_its_bounding_rectangle() {
        let bytes = rect_region([1, 2, 3, 5]);
        let mut r = Reader::new(&bytes);
        let region = Region::read(&mut r).unwrap();
        assert_eq!(r.pos(), 10);
        assert_eq!(
            art(&region, [0, 1, 4, 6]),
            [".....", ".###.", ".###.", "....."]
        );
    }

    #[test]
    fn an_l_shaped_region_holds_exactly_its_pixels() {
        let bytes = inversion_region([0, 0, 4, 4], &[(0, &[0, 4]), (2, &[2, 4]), (4, &[0, 2])]);
        let region = read(&bytes).unwrap();
        assert_eq!(
            art(&region, [0, 0, 5, 5]),
            ["####.", "####.", "##...", "##...", "....."]
        );
    }

    #[test]
    fn a_region_with_a_hole_leaves_the_hole_out() {
        let bytes = inversion_region(
            [0, 0, 4, 4],
            &[(0, &[0, 4]), (1, &[1, 3]), (3, &[1, 3]), (4, &[0, 4])],
        );
        let region = read(&bytes).unwrap();
        assert_eq!(
            art(&region, [0, 0, 5, 4]),
            ["####", "#..#", "#..#", "####", "...."]
        );
    }

    #[test]
    fn points_outside_the_bounding_box_are_outside() {
        // The lines claim more than the box allows; the box wins.
        let bytes = inversion_region([0, 0, 2, 2], &[(0, &[0, 4]), (4, &[0, 4])]);
        let region = read(&bytes).unwrap();
        assert_eq!(art(&region, [0, 0, 3, 3]), ["##.", "##.", "..."]);
    }

    #[test]
    fn rows_above_the_first_line_are_outside() {
        let bytes = inversion_region([0, 0, 4, 4], &[(2, &[0, 4]), (3, &[0, 4])]);
        let region = read(&bytes).unwrap();
        assert_eq!(art(&region, [0, 0, 4, 4]), ["....", "....", "####", "...."]);
    }

    #[test]
    fn read_consumes_exactly_the_region() {
        let mut bytes = inversion_region([0, 0, 2, 2], &[(0, &[0, 2]), (2, &[0, 2])]);
        let len = bytes.len();
        bytes.extend([0xAA, 0xBB]);
        let mut r = Reader::new(&bytes);
        Region::read(&mut r).unwrap();
        assert_eq!(r.pos(), len);
    }

    #[test]
    fn malformed_regions_are_bad_regions_at_their_start() {
        let odd = inversion_region([0, 0, 2, 2], &[(0, &[0, 2, 3]), (2, &[0, 2, 3])]);
        let mut unterminated = inversion_region([0, 0, 2, 2], &[(0, &[0, 2]), (2, &[0, 2])]);
        // Drop the final 0x7FFF but keep the size honest.
        unterminated.truncate(unterminated.len() - 2);
        let size = unterminated.len() as u16;
        unterminated[..2].copy_from_slice(&size.to_be_bytes());
        let lines_out_of_order = inversion_region([0, 0, 2, 2], &[(2, &[0, 2]), (0, &[0, 2])]);
        let points_out_of_order = inversion_region([0, 0, 2, 2], &[(0, &[2, 0]), (2, &[0, 2])]);
        let repeated_line = inversion_region([0, 0, 2, 2], &[(0, &[0, 2]), (0, &[0, 2])]);
        let mut past_the_data = rect_region([0, 0, 2, 2]);
        past_the_data[1] = 12;
        let small = [0, 9, 0, 0, 0, 0, 0, 0, 0, 0];
        for (bytes, name) in [
            (&odd[..], "odd points"),
            (&unterminated, "no terminator"),
            (&lines_out_of_order, "lines out of order"),
            (&points_out_of_order, "points out of order"),
            (&repeated_line, "a repeated line"),
            (&past_the_data, "past the data"),
            (&small, "under 10"),
        ] {
            let size = u16::from_be_bytes([bytes[0], bytes[1]]);
            let mut padded = vec![0xEE; 3];
            padded.extend_from_slice(bytes);
            let mut r = Reader::new(&padded);
            r.skip(3).unwrap();
            assert_eq!(
                Region::read(&mut r),
                Err(GraphicsError::BadRegion { size, offset: 3 }),
                "{name}"
            );
        }
    }

    #[test]
    fn a_missing_size_word_is_an_unexpected_end() {
        assert_eq!(read(&[0]), Err(GraphicsError::UnexpectedEnd { offset: 0 }));
    }
}
