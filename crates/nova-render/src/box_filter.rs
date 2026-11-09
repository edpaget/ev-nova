//! A box filter that resizes an image with transparent pixels: each
//! destination pixel is the average of the source pixels it covers,
//! weighted by the area of each it covers, in premultiplied RGBA.
//!
//! Sprites are drawn with straight alpha, and a decoded sprite's
//! transparent pixels are clear (`[0, 0, 0, 0]`). Averaging straight
//! channels one by one would mix that black into every edge pixel and then
//! fade it by alpha again when drawn, a dark fringe. Averaging the colour
//! premultiplied, then un-premultiplying, keeps an edge's colour and lowers
//! only its alpha; a pixel that covers only clear stays clear. The texels
//! go through Super xBR's premultiplied [`Plane`], which rounds them back
//! the same way.
//!
//! The same filter enlarges: a destination pixel inside one source pixel
//! is that pixel, and one straddling two blends them by the share of each.

use nova_data::graphics::Image;

use crate::super_xbr::{Plane, Texel};

/// `image` resized to `width` x `height` by the box filter (see the module
/// docs).
pub(crate) fn resized(image: &Image, width: u32, height: u32) -> Image {
    let source = Plane::premultiplied(image);
    let columns = shares(image.width(), width);
    let rows = shares(image.height(), height);
    Plane::from_fn(width as usize, height as usize, 1, |x, y| {
        let mut sum: Texel = [0.0; 4];
        for &(sy, wy) in &rows[y] {
            for &(sx, wx) in &columns[x] {
                let texel = source.texel(sx, sy);
                for (total, channel) in sum.iter_mut().zip(texel) {
                    *total += channel * wx * wy;
                }
            }
        }
        sum
    })
    .straight()
}

/// For each of `dst` cells laid over `src` cells along one axis, the
/// source cells it covers, each with the share of the destination cell it
/// covers. The shares of a cell sum to 1, or it has none when `src` is 0.
fn shares(src: u32, dst: u32) -> Vec<Vec<(usize, f32)>> {
    // In units of 1 / (src * dst): destination cell d spans
    // [d * src, (d + 1) * src), source cell s spans [s * dst, (s + 1) * dst).
    let (src, dst) = (u64::from(src), u64::from(dst));
    (0..dst)
        .map(|d| {
            let (start, end) = (d * src, (d + 1) * src);
            (start / dst..end.div_ceil(dst).min(src))
                .map(|s| {
                    let covered = end.min((s + 1) * dst) - start.max(s * dst);
                    (s as usize, covered as f32 / src as f32)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_shrink_takes_whole_cells_evenly() {
        assert_eq!(
            shares(4, 2),
            vec![vec![(0, 0.5), (1, 0.5)], vec![(2, 0.5), (3, 0.5)]]
        );
    }

    #[test]
    fn an_inexact_shrink_shares_the_straddled_cell() {
        let third = 1.0 / 3.0;
        assert_eq!(
            shares(3, 2),
            vec![
                vec![(0, 2.0 * third), (1, third)],
                vec![(1, third), (2, 2.0 * third)]
            ]
        );
    }

    #[test]
    fn an_enlarge_takes_one_cell_or_blends_the_two_it_straddles() {
        assert_eq!(
            shares(2, 3),
            vec![vec![(0, 1.0)], vec![(0, 0.5), (1, 0.5)], vec![(1, 1.0)]]
        );
        assert_eq!(shares(1, 2), vec![vec![(0, 1.0)], vec![(0, 1.0)]]);
    }

    #[test]
    fn the_same_size_is_each_cell_alone() {
        assert_eq!(
            shares(3, 3),
            vec![vec![(0, 1.0)], vec![(1, 1.0)], vec![(2, 1.0)]]
        );
    }

    #[test]
    fn no_source_leaves_every_cell_empty_and_clear() {
        assert_eq!(shares(0, 2), vec![Vec::<(usize, f32)>::new(); 2]);
        let empty = Image::from_rgba(0, 0, Vec::new()).unwrap();
        let out = resized(&empty, 1, 1);
        assert_eq!(out.pixel(0, 0), Some([0; 4]));
    }

    #[test]
    fn a_two_by_two_averages_both_axes_premultiplied() {
        // Red, clear over clear, half-alpha blue, each a quarter.
        let image = Image::from_rgba(
            2,
            2,
            [[255, 0, 0, 255], [0; 4], [0; 4], [0, 0, 255, 128]].concat(),
        )
        .unwrap();
        let out = resized(&image, 1, 1);
        // Premultiplied: r 0.25, b about 0.125, a about 0.375.
        assert_eq!(out.pixel(0, 0), Some([170, 0, 85, 96]));
    }
}
