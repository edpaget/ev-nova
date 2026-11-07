//! QuickDraw's stretch, which the original draws every picture through
//! when its rectangle isn't the picture's own size: `_DrawPicHandle`'s
//! `DT_DrawImage` copies the picture with `CopyBits` or `CopyDeepMask`
//! (`AmbrosiaTools` @0x4780c, @0x47708), and `StretchBits` takes any source
//! and destination of different sizes (`BlitCase`, lines 1231-1238). Line
//! numbers are Apple's `QuickDraw/Stretch.a` (the `boot3` mac-rom source)
//! with its CR line ends read as LF. The game runs Mac OS X's
//! `QD.framework`, a port of it that can't be read, so the classic source
//! is the evidence.
//!
//! The axes have rules of their own, both for a source deeper than one
//! bit, as every stock picture is:
//!
//! - **Rows** ([`rows`]): a DDA. The error starts at `-(src >> 1)`, or at
//!   `-(src - 1)` for an exact shrink (lines 2758-2779). Fetching a row
//!   reads source rows, adding the destination's height for each, until
//!   the error is above 0 (`DoSrcMergeCase`, lines 2896-2908); each row
//!   drawn takes the source's height off, and the next row is fetched once
//!   the error is below 0 (`NODRAW`, lines 3165-3176). So 2 rows to 3
//!   show 0, 0, 1, where sampling at pixel centres shows 0, 1, 1.
//! - **Columns** ([`columns`]): with the ratio from `FixRatio`, which
//!   truncates, the error starts at half of it. Expanding (`PIXSTR`, lines
//!   7842-7866), each column takes the current source column, then the
//!   ratio of source over destination is added in 16 bits, and a carry
//!   moves to the next. Shrinking (`AverageH32`, line 7592), the ratio of
//!   destination over source is added before each column: a carry with
//!   nothing read copies one column, and otherwise the columns read until
//!   the carry merge. The last span stops at the source's last column,
//!   where the original reads past the row.
//! - **Merging** ([`stretched`]): merged rows are merged first, into a
//!   buffer as wide as the source, then merged columns, by the source's
//!   depth ([`PixelDepth`], the depth the picture was stored at, as
//!   `_DrawPicHandle` stretches the picture's own image: `AmbrosiaTools`
//!   `__PICT_HandleBits` @0x55967-0x55a22). `NextAverageV16` and
//!   `NextAverageV32` are chosen by depth at line 3855.
//!   - 32-bit: each 8-bit channel averaged, truncated (`MakeAverage`,
//!     lines 3950-3980, `NextAverageV32`, and `AverageH32`, line 7592).
//!   - 16-bit: each 5-bit channel averaged, truncated (`NextAverageV16`,
//!     line 4005, and `AverageH16`, line 7690), then widened to 8 bits as
//!     the decoder widens it ([`widen_five`]).
//!   - Indexed: the colour of the merged pixel with the largest index
//!     (`NDEEP`, lines 3649-3700, and `PIXSHRNK`'s point loop, line
//!     7796ff). The row buffer keeps the chosen index, so the column pass
//!     compares indices too. Equal indices are one colour under one table,
//!     and the first is kept.
//!
//!   A span of one pixel copies it unchanged at every depth. Alpha, which
//!   is a mask's, is averaged on 8 bits at every depth. The original
//!   stretches the mask apart from the picture, through `CopyDeepMask`,
//!   and that merge isn't modelled here.

use nova_data::graphics::{DeepImage, Image, PixelDepth, widen_five};

/// The source rows (or columns) one destination row (or column) shows:
/// `count` of them from `first`, averaged when there are several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The first source index.
    pub first: u32,
    /// How many, from `first`.
    pub count: u32,
}

/// The source rows each of `dst` destination rows shows, stretching `src`
/// rows: the vertical DDA (see the module docs).
#[must_use]
pub fn rows(src: u32, dst: u32) -> Vec<Span> {
    let (s, d) = (i64::from(src), i64::from(dst));
    // `is_multiple_of(0)` holds only for 0, which `src > dst` rules out.
    let exact_shrink = src > dst && src.is_multiple_of(dst);
    let mut error = if exact_shrink { -(s - 1) } else { -(s >> 1) };
    let mut next = 0;
    let mut map = Vec::with_capacity(usize::try_from(dst).unwrap_or(0));
    while map.len() < map.capacity() {
        let first = next;
        while next < src {
            next += 1;
            error += d;
            if error > 0 {
                break;
            }
        }
        let span = Span {
            first,
            count: next - first,
        };
        loop {
            map.push(span);
            if map.len() == map.capacity() {
                break;
            }
            error -= s;
            if error < 0 {
                break;
            }
        }
    }
    map
}

/// The source columns each of `dst` destination columns shows, stretching
/// `src` columns (see the module docs).
///
/// A shrink's last span ends at the source's last column. `FixRatio`
/// truncates, so the carries come late and the last span would run past
/// the row (190 to 1 would read 191 columns, 1070 to 32 end at 1071); the
/// original reads past it, the engine doesn't. The truncation loses less
/// than one column's worth of carry over the whole row, so for a source
/// up to 32,768 wide only the last span runs over, and every span starts
/// inside the row. [`rows`] needs no
/// bound of its own, as its fetch stops at the source's last row.
#[must_use]
pub fn columns(src: u32, dst: u32) -> Vec<Span> {
    if src == dst {
        return (0..src).map(|first| Span { first, count: 1 }).collect();
    }
    let count = usize::try_from(dst).unwrap_or(0);
    let mut map = Vec::with_capacity(count);
    if src < dst {
        let ratio = fix_ratio(src, dst);
        let mut error = ratio >> 1;
        let mut first = 0;
        for _ in 0..dst {
            map.push(Span { first, count: 1 });
            error += ratio;
            if error >= CARRY {
                error -= CARRY;
                first += 1;
            }
        }
        return map;
    }
    let ratio = fix_ratio(dst, src);
    let mut error = ratio >> 1;
    let mut next = 0;
    while map.len() < count {
        let first = next;
        error += ratio;
        if error < CARRY {
            loop {
                next += 1;
                error += ratio;
                if error >= CARRY {
                    break;
                }
            }
        }
        error -= CARRY;
        next += 1;
        map.push(Span {
            first,
            count: next.min(src) - first,
        });
    }
    map
}

/// The carry out of a 16-bit add.
const CARRY: u32 = 1 << 16;

/// `FixRatio(numerator, denominator)`'s fraction, below 1.0, as 16 bits:
/// truncated (`Munger.a`'s `FixRatio` divides with `DIVS.L`).
fn fix_ratio(numerator: u32, denominator: u32) -> u32 {
    u32::try_from((u64::from(numerator) << 16) / u64::from(denominator)).unwrap_or(0)
}

/// Whether drawing a `src` picture at `dst` by the GPU's sampling would
/// show other pixels than QuickDraw's stretch: unless each axis keeps its
/// size or repeats every source row or column the same whole number of
/// times, as the GPU's sampling at pixel centres then does too.
#[must_use]
pub fn differs(src: (u32, u32), dst: (u32, u32)) -> bool {
    !(repeats_alike(&columns(src.0, dst.0), src.0, dst.0)
        && repeats_alike(&rows(src.1, dst.1), src.1, dst.1))
}

/// Whether `map` shows each of `src` source indices `dst / src` times in
/// turn. An axis of none kept at none shows nothing, so nothing other
/// (`is_multiple_of(0)` holds only for 0, and the empty map divides by
/// nothing).
fn repeats_alike(map: &[Span], src: u32, dst: u32) -> bool {
    dst.is_multiple_of(src)
        && map.iter().zip(0..).all(|(span, d): (&Span, u32)| {
            *span
                == Span {
                    first: d / (dst / src),
                    count: 1,
                }
        })
}

/// `image` stretched to `width` x `height` as QuickDraw stretches it: its
/// rows by [`rows`], merged ones merged by the picture's depth, then its
/// columns by [`columns`], likewise (see the module docs' "Merging").
#[must_use]
pub fn stretched(image: &DeepImage, width: u32, height: u32) -> Image {
    let depth = image.depth();
    let source = image.image();
    let source_width = source.width();
    let index = |x: u32, y: u32| match depth {
        PixelDepth::Indexed { indices } => indices[y as usize * source_width as usize + x as usize],
        PixelDepth::Direct16 | PixelDepth::Direct32 => 0,
    };
    let rows = rows(source.height(), height);
    let mut tall = Vec::with_capacity(rows.len() * source_width as usize);
    for span in &rows {
        for x in 0..source_width {
            tall.push(merge(
                depth,
                (span.first..span.first + span.count).map(|y| (pixel(source, x, y), index(x, y))),
            ));
        }
    }
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    let columns = columns(source_width, width);
    for row in tall
        .chunks(source_width.max(1) as usize)
        .take(height as usize)
    {
        for span in &columns {
            let (rgba, _) = merge(
                depth,
                row[span.first as usize..(span.first + span.count) as usize]
                    .iter()
                    .copied(),
            );
            out.extend(rgba);
        }
    }
    Image::from_rgba(width, height, out).expect("sized by its rows and columns")
}

/// The pixel at (`x`, `y`) of `image`.
fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * image.width() as usize + x as usize) * 4;
    let p = &image.pixels()[at..at + 4];
    [p[0], p[1], p[2], p[3]]
}

/// `pixels` (each a colour and its index) merged into one by `depth`. One
/// pixel is copied unchanged. Several average their alpha on 8 bits, and
/// their colours: on 8-bit channels at [`PixelDepth::Direct32`], on 5-bit
/// ones widened back at [`PixelDepth::Direct16`], or as the pixel with the
/// largest index (the first of equals) at [`PixelDepth::Indexed`].
fn merge(depth: &PixelDepth, pixels: impl Iterator<Item = ([u8; 4], u8)>) -> ([u8; 4], u8) {
    let pixels: Vec<([u8; 4], u8)> = pixels.collect();
    if let [one] = pixels[..] {
        return one;
    }
    let colours = || pixels.iter().map(|&(rgba, _)| rgba);
    let [_, _, _, alpha] = average(colours(), |channel| channel);
    match depth {
        PixelDepth::Direct32 => (average(colours(), |channel| channel), 0),
        PixelDepth::Direct16 => {
            let [r, g, b, _] = average(colours(), |channel| channel >> 3);
            ([widen_five(r), widen_five(g), widen_five(b), alpha], 0)
        }
        PixelDepth::Indexed { .. } => {
            let (rgba, largest) = pixels
                .iter()
                .copied()
                .reduce(|kept, next| if next.1 > kept.1 { next } else { kept })
                .unwrap_or_default();
            ([rgba[0], rgba[1], rgba[2], alpha], largest)
        }
    }
}

/// The average of `pixels`' channels, each read through `channel` and
/// truncated.
fn average(pixels: impl Iterator<Item = [u8; 4]>, channel: impl Fn(u8) -> u8) -> [u8; 4] {
    let mut sum = [0_u32; 4];
    let mut count = 0_u32;
    for pixel in pixels {
        for (total, value) in sum.iter_mut().zip(pixel) {
            *total += u32::from(channel(value));
        }
        count += 1;
    }
    sum.map(|total| u8::try_from(total / count.max(1)).unwrap_or(u8::MAX))
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::decode_pict_deep;
    use nova_data::graphics::fixture::{Ctab, DirectBits, IndexedBits, PictBuilder, rgb888_pict};

    use super::*;

    fn one(first: u32) -> Span {
        Span { first, count: 1 }
    }

    fn firsts(spans: &[Span]) -> Vec<u32> {
        spans.iter().map(|span| span.first).collect()
    }

    #[test]
    fn two_rows_stretched_to_three_repeat_the_first() {
        assert_eq!(rows(2, 3), [one(0), one(0), one(1)]);
        assert_eq!(columns(2, 3), [one(0), one(0), one(1)]);
    }

    #[test]
    fn two_hundred_and_forty_four_rows_to_two_hundred_and_forty_five_repeat_row_121() {
        let map = rows(244, 245);
        assert_eq!(map.len(), 245);
        let repeated: Vec<usize> = (1..245).filter(|&d| map[d] == map[d - 1]).collect();
        assert_eq!(repeated, [122]);
        assert_eq!(map[122], one(121));
        assert_eq!(map[208], one(207), "picture row 207 lands at row 208");
        assert_eq!(map[207], one(206));
    }

    #[test]
    fn six_hundred_and_forty_nine_columns_to_six_hundred_and_fifty_one() {
        let map = columns(649, 651);
        assert_eq!(map.len(), 651);
        let repeated: Vec<usize> = (1..651).filter(|&d| map[d] == map[d - 1]).collect();
        assert_eq!(repeated, [162, 487]);
        assert_eq!(firsts(&map[160..164]), [160, 161, 161, 162]);
        assert_eq!(firsts(&map[485..489]), [484, 485, 485, 486]);
        assert_eq!(map[650], one(648));
    }

    #[test]
    fn an_exact_multiple_repeats_each_source_row_and_column_alike() {
        let repeated =
            |n: u32, times: u32| (0..n * times).map(|d| one(d / times)).collect::<Vec<_>>();
        assert_eq!(rows(4, 8), repeated(4, 2));
        assert_eq!(rows(1, 5), repeated(1, 5));
        assert_eq!(columns(4, 8), repeated(4, 2));
        assert_eq!(columns(1, 5), repeated(1, 5));
        assert!(!differs((4, 1), (8, 5)));
    }

    #[test]
    fn the_same_size_is_the_identity() {
        let identity = (0..7).map(one).collect::<Vec<_>>();
        assert_eq!(rows(7, 7), identity);
        assert_eq!(columns(7, 7), identity);
        assert!(!differs((7, 7), (7, 7)));
    }

    /// An exact shrink starts the error at `-(src - 1)` (lines 2762-2770):
    /// six rows to two merge three each, where the general start would
    /// merge two, then three.
    #[test]
    fn an_exact_shrink_of_rows_merges_them_evenly() {
        let span = |first, count| Span { first, count };
        assert_eq!(rows(6, 2), [span(0, 3), span(3, 3)]);
    }

    /// Five rows to two: the general start, `-(src >> 1)`, merges two
    /// then two, leaving the last; five columns to two merge three then
    /// two (`AverageH32`).
    #[test]
    fn an_inexact_shrink_merges_rows_and_columns_by_their_own_rules() {
        let span = |first, count| Span { first, count };
        assert_eq!(rows(5, 2), [span(0, 2), span(2, 2)]);
        assert_eq!(columns(5, 2), [span(0, 3), span(3, 2)]);
        assert_eq!(columns(649, 648)[323], span(323, 2));
        assert!(differs((5, 1), (2, 1)));
    }

    /// `FixRatio(121, 195)` is 40665, odd, so the error starts at 20332,
    /// and at column 111 the add lands on 0x10000 exactly: the 16-bit add
    /// carries with no column read yet, so `AverageH32` copies one column
    /// (`@1to1`) rather than merging two.
    #[test]
    fn a_shrinks_carry_onto_the_16_bit_boundary_copies_one_column() {
        let span = |first, count| Span { first, count };
        assert_eq!(
            columns(195, 121)[110..113],
            [span(177, 2), span(179, 1), span(180, 2)]
        );
    }

    /// Whether every span of `map` reads one or more of `src` indices and
    /// none past them.
    fn within(map: &[Span], src: u32) -> bool {
        map.iter()
            .all(|span| span.count >= 1 && span.first + span.count <= src)
    }

    /// `FixRatio` truncates, so a shrink's last span would read columns
    /// past the source (190 to 1 ends at 191): the engine bounds it by the
    /// source, where the original reads past the row.
    #[test]
    fn every_span_lies_within_the_source() {
        for (src, dst) in [(190, 1), (276, 2), (325, 3), (1070, 32), (1100, 1)] {
            assert!(within(&columns(src, dst), src), "columns {src} to {dst}");
            assert!(within(&rows(src, dst), src), "rows {src} to {dst}");
        }
        let small = (1..=300).flat_map(|src| (1..=300).map(move |dst| (src, dst)));
        let wide = (300..=1100).flat_map(|src| (1..=40).map(move |dst| (src, dst)));
        for (src, dst) in small.chain(wide) {
            let (cols, rows) = (columns(src, dst), rows(src, dst));
            assert_eq!((cols.len(), rows.len()), (dst as usize, dst as usize));
            assert!(within(&cols, src), "columns {src} to {dst}");
            assert!(within(&rows, src), "rows {src} to {dst}");
        }
    }

    /// The bounded last span merges every column left: 190 to 1 averages
    /// all 190.
    #[test]
    fn a_shrinks_last_span_takes_the_columns_left() {
        assert_eq!(
            columns(190, 1),
            [Span {
                first: 0,
                count: 190
            }]
        );
    }

    #[test]
    fn a_wide_picture_shrunk_to_one_pixel_reads_only_its_own_row() {
        let picture = deep32(image(300, 1, &[[8, 0, 0, 255]; 300]));
        assert_eq!(stretched(&picture, 1, 1), image(1, 1, &[[8, 0, 0, 255]]));
    }

    #[test]
    fn a_zero_wide_axis_kept_at_zero_shows_nothing_other() {
        assert!(!differs((0, 2), (0, 4)));
    }

    #[test]
    fn a_stretch_differs_from_the_gpus_unless_every_pixel_repeats_alike() {
        assert!(differs((2, 2), (2, 3)), "rows 0, 0, 1 against 0, 1, 1");
        assert!(differs((2, 2), (3, 2)), "columns likewise");
        assert!(!differs((2, 2), (4, 6)), "exact on both axes");
    }

    fn image(width: u32, height: u32, pixels: &[[u8; 4]]) -> Image {
        Image::from_rgba(width, height, pixels.concat()).expect("sized")
    }

    fn deep32(image: Image) -> DeepImage {
        DeepImage::new(image, PixelDepth::Direct32).expect("direct")
    }

    #[test]
    fn a_two_by_two_picture_stretched_to_three_by_three() {
        let [a, b, c, d] = [
            [1, 0, 0, 255],
            [2, 0, 0, 255],
            [3, 0, 0, 255],
            [4, 0, 0, 255],
        ];
        let picture = deep32(image(2, 2, &[a, b, c, d]));
        assert_eq!(
            stretched(&picture, 3, 3),
            image(3, 3, &[a, a, b, a, a, b, c, c, d])
        );
    }

    /// Rows merge first, each channel averaged and truncated
    /// (`MakeAverage` lines 3950-3980), then columns (`AverageH32`).
    #[test]
    fn a_shrink_averages_rows_then_columns_truncating() {
        let picture = deep32(image(
            2,
            2,
            &[
                [0, 10, 200, 255],
                [1, 11, 201, 255],
                [1, 20, 100, 0],
                [2, 21, 101, 0],
            ],
        ));
        // Rows: (0+1)/2 = 0, (1+2)/2 = 1; then columns: (0+1)/2 = 0.
        assert_eq!(stretched(&picture, 1, 1), image(1, 1, &[[0, 15, 150, 127]]));
    }

    /// `pict` decoded with its depth, as the renderer gets it.
    fn decoded(pict: &[u8]) -> DeepImage {
        decode_pict_deep(pict).expect("decodes")
    }

    /// A 16-bit picture `width` x `height` of `words`.
    fn pict_16(width: i16, height: i16, words: &[u16]) -> DeepImage {
        let bounds = [0, 0, height, width];
        decoded(
            &PictBuilder::new(bounds)
                .direct_bits(&DirectBits::rgb555(bounds, words))
                .end()
                .build(),
        )
    }

    /// The word of 5-bit channels `r`, `g`, `b`.
    fn word(r: u16, g: u16, b: u16) -> u16 {
        (r << 10) | (g << 5) | b
    }

    /// One pixel decoded from a 16-bit `word`: what the 5-bit average
    /// shows, widened by the decoder's own rule.
    fn shown_16(word: u16) -> Image {
        pict_16(1, 1, &[word]).into_image()
    }

    /// AC1: two rows of 5-bit channels 1 and 2 (decoded 8 and 16) merge
    /// to 5-bit 1 (`NextAverageV16`, line 4005), shown as 8, where the
    /// 8-bit average would be 12.
    #[test]
    fn a_16_bit_shrink_of_two_rows_averages_5_bit_channels() {
        let picture = pict_16(1, 2, &[word(1, 3, 30), word(2, 4, 31)]);
        assert_eq!(picture.depth(), &PixelDepth::Direct16);
        let shrunk = stretched(&picture, 1, 1);
        assert_eq!(shrunk, shown_16(word(1, 3, 30)));
        let eight_bit = stretched(&deep32(picture.image().clone()), 1, 1);
        assert_eq!(eight_bit.pixel(0, 0).map(|p| p[0]), Some(12));
        assert_ne!(shrunk, eight_bit);
    }

    /// Columns merge on 5-bit channels too (`AverageH16`, line 7690).
    #[test]
    fn a_16_bit_shrink_of_two_columns_averages_5_bit_channels() {
        let picture = pict_16(2, 1, &[word(1, 3, 30), word(2, 4, 31)]);
        assert_eq!(stretched(&picture, 1, 1), shown_16(word(1, 3, 30)));
    }

    /// Three rows of 1, 1, 2 average to 4 / 3, truncated to 1 (shown as
    /// 8), where the 8-bit average of 8, 8, 16 is 10.
    #[test]
    fn a_16_bit_shrink_of_three_truncates() {
        let picture = pict_16(1, 3, &[word(1, 0, 0), word(1, 0, 0), word(2, 31, 0)]);
        assert_eq!(stretched(&picture, 1, 1), shown_16(word(1, 10, 0)));
    }

    /// A 16-bit picture's alpha (a mask's) still averages on 8 bits.
    #[test]
    fn a_16_bit_shrinks_alpha_averages_on_8_bits() {
        let picture = pict_16(1, 2, &[word(4, 4, 4); 2]);
        let mut pixels = picture.image().pixels().to_vec();
        (pixels[3], pixels[7]) = (255, 0);
        let masked = picture
            .with_image(Image::from_rgba(1, 2, pixels).expect("1 x 2"))
            .expect("same size");
        let shrunk = stretched(&masked, 1, 1);
        assert_eq!(shrunk.pixel(0, 0).map(|p| p[3]), Some(127));
    }

    /// AC2: a 32-bit picture's two rows still average 8-bit channels,
    /// truncated (`NextAverageV32`).
    #[test]
    fn a_32_bit_shrink_of_two_rows_averages_8_bit_channels() {
        let picture = decoded(&rgb888_pict(1, 2, &[[10, 100, 7], [13, 201, 8]]));
        assert_eq!(picture.depth(), &PixelDepth::Direct32);
        assert_eq!(stretched(&picture, 1, 1), image(1, 1, &[[11, 150, 7, 255]]));
    }

    /// An 8-bit picture whose larger indices are darker: 0 white, 1 light
    /// grey, 2 dark grey, 3 black.
    fn pict_indexed(width: i16, height: i16, indices: &[u8]) -> DeepImage {
        let ctab = Ctab {
            device: false,
            entries: vec![
                (0, [0xFFFF; 3]),
                (1, [0xC000; 3]),
                (2, [0x4000; 3]),
                (3, [0; 3]),
            ],
        };
        let bounds = [0, 0, height, width];
        decoded(
            &PictBuilder::new(bounds)
                .packbits_rect(&IndexedBits::new(bounds, 8, &ctab, indices))
                .end()
                .build(),
        )
    }

    /// The colour of index `index` in [`pict_indexed`]'s table.
    fn shown_indexed(index: u8) -> Image {
        pict_indexed(1, 1, &[index]).into_image()
    }

    /// AC3: a 2 x 2 shrink keeps the largest index's colour (`NDEEP`,
    /// lines 3649-3700, and `PIXSHRNK`, line 7796ff): not the average,
    /// and not the brightest.
    #[test]
    fn an_indexed_shrink_keeps_the_largest_indexs_colour() {
        let picture = pict_indexed(2, 2, &[0, 2, 1, 0]);
        assert!(matches!(picture.depth(), PixelDepth::Indexed { .. }));
        assert_eq!(stretched(&picture, 1, 1), shown_indexed(2));
    }

    /// The row pass keeps each merged pixel's index, so the column pass
    /// compares indices: 3 in the second row, second column, wins over the
    /// first row's 2 in the first column.
    #[test]
    fn an_indexed_shrinks_index_survives_the_row_pass() {
        let picture = pict_indexed(2, 2, &[2, 0, 1, 3]);
        assert_eq!(stretched(&picture, 1, 1), shown_indexed(3));
        let picture = pict_indexed(2, 2, &[0, 1, 3, 0]);
        assert_eq!(stretched(&picture, 1, 1), shown_indexed(3));
        let rows_only = pict_indexed(2, 2, &[1, 0, 0, 2]);
        assert_eq!(
            stretched(&rows_only, 2, 1),
            image(
                2,
                1,
                &[
                    shown_indexed(1).pixel(0, 0).expect("one"),
                    shown_indexed(2).pixel(0, 0).expect("one"),
                ]
            )
        );
    }

    /// An indexed picture's alpha (a mask's) averages on 8 bits.
    #[test]
    fn an_indexed_shrinks_alpha_averages_on_8_bits() {
        let picture = pict_indexed(2, 1, &[3, 1]);
        let mut pixels = picture.image().pixels().to_vec();
        (pixels[3], pixels[7]) = (0, 201);
        let masked = picture
            .with_image(Image::from_rgba(2, 1, pixels).expect("2 x 1"))
            .expect("same size");
        assert_eq!(stretched(&masked, 1, 1).pixel(0, 0), Some([0, 0, 0, 100]));
    }

    /// An expansion repeats pixels unchanged at every depth, colours a
    /// 16-bit decode would never make included.
    #[test]
    fn an_expansion_copies_pixels_unchanged_at_every_depth() {
        let odd = image(2, 1, &[[1, 2, 3, 4], [250, 6, 7, 8]]);
        let sixteen = pict_16(2, 1, &[0, 0])
            .with_image(odd.clone())
            .expect("same size");
        let indexed = pict_indexed(2, 1, &[1, 0])
            .with_image(odd.clone())
            .expect("same size");
        let expected = image(3, 1, &[[1, 2, 3, 4], [1, 2, 3, 4], [250, 6, 7, 8]]);
        for picture in [sixteen, indexed, deep32(odd)] {
            assert_eq!(stretched(&picture, 3, 1), expected, "{:?}", picture.depth());
        }
    }
}
