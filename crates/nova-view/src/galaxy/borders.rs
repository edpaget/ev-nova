//! The maps' government borders, as `_DrawMapInfluence` paints them
//! (@0xe360-0xe567): each lattice cell [`nova_sim::map_borders`] works
//! out, an opaque square of 2 × step pixels in its strongest government's
//! colour at half the cell's value.
//!
//! **A cell's square** runs from (left + 2x, top + 2y) of the map item to
//! 2 × step pixels on; a square wholly outside the item is skipped, each
//! edge compared inclusively (@0xe405-0xe433), and any other is clamped to
//! the item (@0xe439-0xe48d), so one only touching an edge paints nothing.
//! The grid's offset is a quarter of the item's size, truncated, so on the
//! in-flight maps the system at the scroll is cell (114, 105), drawn at x
//! 221 + 228 = 449, a pixel left of where `_DrawMap` puts it, 450.
//!
//! **Its colour** is the government's `gövt` `Color` (+0x7c/+0x7e/+0x80,
//! 8 bits a channel, @0x7bd38-0x7bd58), each channel × (value × 0.5),
//! truncated, as a 16-bit `RGBColor` component (@0xe4b6-0xe50a), which a
//! direct screen shows as its high byte ([`shade`]): about half the colour
//! at 255. A government with no colour reads 0. Each square is its own
//! `PaintRect`, as the original paints them.

use std::collections::BTreeMap;

use nova_sim::map_borders::PaintedCell;

use super::catalog::GovtId;
use super::view::Bounds;
use crate::draw::fill_rect;
use crate::flight::radar::IRect;
use crate::{Color, DrawList, Point};

/// What a cell's value is halved by before it scales a channel (0.5,
/// 0xdd128).
pub const SHADE: f64 = 0.5;

/// `channel`, an 8-bit `gövt` colour channel, at a cell's `value`: the
/// high byte of the 16-bit component trunc(channel × (value × 0.5)).
#[must_use]
pub fn shade(channel: u8, value: u8) -> u8 {
    let component = (f64::from(channel) * (f64::from(value) * SHADE)).trunc();
    // At most 255 × 127.5: a 16-bit component, shown as its high byte.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let component = component as u16;
    component.to_be_bytes()[0]
}

/// Paints `cells` as `_DrawMapInfluence` does (see the module docs): each
/// at `origin` + 2 × its place, clamped to `area` or skipped outside it,
/// in its government's colour from `colours` (a 24-bit `00RRGGBB` value)
/// shaded by its value.
pub fn draw_cells(
    list: &mut DrawList,
    cells: &[PaintedCell],
    origin: (i32, i32),
    area: IRect,
    colours: &BTreeMap<GovtId, u32>,
) {
    for cell in cells {
        let left = origin.0 + 2 * cell.x;
        let top = origin.1 + 2 * cell.y;
        let side = 2 * cell.step;
        let (right, bottom) = (left + side, top + side);
        if right < area.left || left > area.right || bottom < area.top || top > area.bottom {
            continue;
        }
        let (left, top) = (left.max(area.left), top.max(area.top));
        let (right, bottom) = (right.min(area.right), bottom.min(area.bottom));
        if left >= right || top >= bottom {
            continue;
        }
        let raw = colours.get(&cell.govt).copied().unwrap_or(0);
        let [_, r, g, b] = raw.to_be_bytes();
        let color = Color::rgba(
            shade(r, cell.value),
            shade(g, cell.value),
            shade(b, cell.value),
            255,
        );
        #[allow(clippy::cast_precision_loss)] // Screen pixels.
        let square = Bounds {
            min: Point::new(left as f32, top as f32),
            max: Point::new(right as f32, bottom as f32),
        };
        fill_rect(list, square, color);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;
    use crate::galaxy::dialog::map_area;

    /// The rects `list` fills, as (left, top, right, bottom, colour).
    pub(crate) fn rects(list: &DrawList) -> Vec<(f32, f32, f32, f32, Color)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if from.y == to.y => Some((
                    from.x.min(to.x),
                    from.y - width / 2.0,
                    from.x.max(to.x),
                    from.y + width / 2.0,
                    color,
                )),
                _ => None,
            })
            .collect()
    }

    /// The in-flight maps' item 3, (221, 136)-(679, 556).
    fn item_3() -> IRect {
        IRect::from_bounds(map_area())
    }

    const FED: GovtId = GovtId(128);

    fn cell(x: i32, y: i32, step: i32, value: u8) -> PaintedCell {
        PaintedCell {
            x,
            y,
            step,
            govt: FED,
            value,
        }
    }

    fn colours() -> BTreeMap<GovtId, u32> {
        BTreeMap::from([(FED, 0x00CF_FF10)])
    }

    fn drawn(cells: &[PaintedCell]) -> Vec<(f32, f32, f32, f32, Color)> {
        let mut list = DrawList::new();
        draw_cells(&mut list, cells, (221, 136), item_3(), &colours());
        rects(&list)
    }

    #[test]
    fn a_channel_is_shaded_by_half_the_value_and_shown_as_its_high_byte() {
        // 207 × 127.5 = 26 392.5: 26 392, 0x6718.
        assert_eq!(shade(0xcf, 255), 103);
        assert_eq!(shade(255, 255), 127, "32 512, 0x7f00");
        assert_eq!(shade(255, 1), 0, "127, under a byte");
        assert_eq!(shade(0, 255), 0);
        // 255 × 64 = 16 320, 0x3fc0.
        assert_eq!(shade(255, 128), 63);
        // 16 × 101.5 = 1624, 0x0658.
        assert_eq!(shade(16, 203), 6);
    }

    #[test]
    fn a_fine_cell_is_a_2_pixel_square_a_pixel_left_of_the_maps_centre() {
        assert_eq!(
            drawn(&[cell(114, 105, 1, 255)]),
            [(449.0, 346.0, 451.0, 348.0, Color::rgba(103, 127, 7, 255))]
        );
        assert_eq!(
            crate::galaxy::dialog::map_centre(),
            Point::new(450.0, 346.0)
        );
    }

    #[test]
    fn a_coarse_cell_is_a_16_pixel_square() {
        assert_eq!(
            drawn(&[cell(112, 104, 8, 128)]),
            [(445.0, 344.0, 461.0, 360.0, Color::rgba(51, 63, 4, 255))]
        );
    }

    #[test]
    fn a_square_across_an_edge_is_clamped_to_item_3() {
        // x 221 + 2 × 224 = 669 to 685, past the right edge at 679; y 136
        // − 4 to 12 past the top.
        assert_eq!(
            drawn(&[cell(224, 0, 8, 255), cell(0, -2, 8, 255)]),
            [
                (669.0, 136.0, 679.0, 152.0, Color::rgba(103, 127, 7, 255)),
                (221.0, 136.0, 237.0, 148.0, Color::rgba(103, 127, 7, 255)),
            ]
        );
    }

    #[test]
    fn a_square_wholly_outside_is_skipped_and_one_touching_an_edge_paints_nothing() {
        // 221 + 2 × 232 = 685: past 679. 221 + 2 × 229 = 679: touching.
        let mut list = DrawList::new();
        let cells = [
            cell(232, 0, 1, 255),
            cell(229, 0, 1, 255),
            cell(0, 210, 1, 255),
            cell(-1, 0, 1, 255),
            cell(0, -1, 1, 255),
            cell(-2, 0, 1, 255),
            cell(0, -2, 1, 255),
        ];
        draw_cells(&mut list, &cells, (221, 136), item_3(), &colours());
        assert!(list.is_empty(), "{:?}", rects(&list));
    }

    #[test]
    fn each_cell_is_its_own_rect_in_its_governments_colour() {
        let mut colours = colours();
        colours.insert(GovtId(129), 0x0000_00FF);
        let mut list = DrawList::new();
        let cells = [
            cell(0, 0, 1, 255),
            cell(0, 1, 1, 255),
            PaintedCell {
                govt: GovtId(129),
                ..cell(0, 2, 1, 255)
            },
            PaintedCell {
                govt: GovtId(130),
                ..cell(0, 3, 1, 255)
            },
        ];
        draw_cells(&mut list, &cells, (221, 136), item_3(), &colours);
        let fed = Color::rgba(103, 127, 7, 255);
        assert_eq!(
            rects(&list),
            [
                (221.0, 136.0, 223.0, 138.0, fed),
                (221.0, 138.0, 223.0, 140.0, fed),
                (221.0, 140.0, 223.0, 142.0, Color::rgba(0, 0, 127, 255)),
                (221.0, 142.0, 223.0, 144.0, Color::BLACK),
            ],
            "one PaintRect a cell; 130 has no colour"
        );
    }
}
