//! A text dialog shrunk to its text, as `_ShowTextDialog` (in `EV
//! Nova.app`) shrinks "Desc Dialog" (`DLOG` 3003) before showing it
//! (@0x5aabe-0x5adee).
//!
//! - `h` is the text's height wrapped to the text item's width (`TENew`
//!   on item 3's rect @0x5a9a1-0x5a9b0, `TEGetHeight` @0x5aa87): its
//!   wrapped lines times the line height.
//! - `boxH` is the text item's height (@0x5aabe-0x5aac2).
//! - The shrink runs only when `h < boxH` (@0x5aac9). Then
//!   `d = boxH − max(h, `[`MIN_TEXT_HEIGHT`]`) − `[`MARGIN`]
//!   (@0x5aade-0x5aaf6): the window's height drops by `d` and its width is
//!   unchanged (`SizeWindow` @0x5ab4c), the moved items (3003's Done and
//!   scroll arrows, items 1, 5 and 6) go up by `d` (@0x5abc3-0x5ad14), and
//!   the text item's bottom goes up by `d` (@0x5ad57-0x5ad73). Every other
//!   item stays where it is.
//! - There is no other cap. `d` is negative when `h` falls between
//!   `boxH − MARGIN` and `boxH` (246 < h < 262 on stock 3003), and then the
//!   dialog grows instead, by up to `MARGIN − 1`: the original's
//!   arithmetic, kept as it is.
//!
//! `_OfferOneMission` shrinks the offer dialog (`DLOG` 1016) by the same
//! rule (@0xa2548-0xa2816), but after `_CenterWindow` has placed it at its
//! stock size, so it then moves the window too ([`shrink_after_placing`]):
//! the new top is `trunc(top + d × `[`WINDOW_MOVE_FACTOR`]`)`, the sum
//! truncated and not the product (`cvttsd2si` @0xa25f5-0xa260c,
//! `MoveWindow` @0xa2640), which differs when `d` is negative: `d = −7`
//! from 226 gives 223, not 224.

use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::text::{TextMetrics, wrap};

use super::dialog::{DialogTemplate, Placement};

/// The least text height the shrink leaves room for: a shorter text keeps
/// a box this tall plus [`MARGIN`] (`cmovge` on 0x30 @0x5aaed).
pub const MIN_TEXT_HEIGHT: f32 = 48.0;

/// The room the shrink leaves under the text in its box (@0x5aaf6).
pub const MARGIN: f32 = 16.0;

/// How much of the shrink `_OfferOneMission` moves the window down by:
/// the double at 0xdd8d0, `0x3FD3333333333333`.
pub const WINDOW_MOVE_FACTOR: f64 = 0.3;

/// `template` shrunk to a text `text_height` high in item `text_item`
/// (from 1), moving the items `moved` (from 1) with its bottom edge, as
/// the module docs say. A template with no `text_item`, or a text at least
/// as tall as its box, comes back unchanged; a `moved` item past the end
/// is ignored.
#[must_use]
pub fn shrink_to_text(
    template: &DialogTemplate,
    text_item: usize,
    moved: &[usize],
    text_height: f32,
) -> DialogTemplate {
    let mut shrunk = template.clone();
    let Some(text) = text_item
        .checked_sub(1)
        .and_then(|index| shrunk.items.get_mut(index))
    else {
        return shrunk;
    };
    let box_height = text.bounds.height();
    if text_height >= box_height {
        return shrunk;
    }
    let d = box_height - text_height.max(MIN_TEXT_HEIGHT) - MARGIN;
    text.bounds.max.y -= d;
    shrunk.bounds.max.y -= d;
    for &number in moved {
        if let Some(item) = number
            .checked_sub(1)
            .and_then(|index| shrunk.items.get_mut(index))
        {
            item.bounds = item.bounds.offset(Point::new(0.0, -d));
        }
    }
    shrunk
}

/// `text`'s height wrapped to `width` in `font` at `size`: what `TENew` on
/// the text item's rect and `TEGetHeight` give, its wrapped lines times the
/// line height.
#[must_use]
pub fn wrapped_height(
    metrics: &dyn TextMetrics,
    font: Font,
    size: f32,
    text: &str,
    width: f32,
) -> f32 {
    wrap(&metrics, font, size, text, width).len() as f32 * metrics.line_height(font, size)
}

/// `template` shrunk as [`shrink_to_text`] shrinks it, after being placed
/// at its unshrunk size, as the module docs say: the result is
/// [`Placement::Fixed`] at the placement's left edge and at
/// `trunc(top + d × `[`WINDOW_MOVE_FACTOR`]`)`. A template the shrink leaves
/// alone stays where its placement puts it.
#[must_use]
pub fn shrink_after_placing(
    template: &DialogTemplate,
    text_item: usize,
    moved: &[usize],
    text_height: f32,
) -> DialogTemplate {
    let origin = template.placement.origin(template.bounds);
    let mut shrunk = shrink_to_text(template, text_item, moved, text_height);
    let d = template.bounds.height() - shrunk.bounds.height();
    let top = (f64::from(origin.y) + f64::from(d) * WINDOW_MOVE_FACTOR).trunc();
    #[allow(clippy::cast_possible_truncation)] // A screen coordinate.
    let top = top as f32;
    shrunk.bounds = Bounds::at(
        Point::new(origin.x, top),
        shrunk.bounds.width(),
        shrunk.bounds.height(),
    );
    shrunk.placement = Placement::Fixed;
    shrunk
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DLOG` 3003 and its `DITL`: 441 x 313, Done (1), the parked
    /// picture (2), the 417 x 262 text box (3), the parked map item (4) and
    /// the two scroll arrows (5, 6).
    fn stock() -> DialogTemplate {
        let item = |bounds| ItemTemplate {
            bounds,
            enabled: true,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 441.0, 313.0),
            placement: Placement::CenterWindow,
            items: vec![
                item(rect(173.0, 281.0, 99.0, 25.0)),
                item(rect(221.0, 329.0, 68.0, 30.0)),
                item(rect(11.0, 10.0, 417.0, 262.0)),
                item(rect(70.0, 345.0, 78.0, 25.0)),
                item(rect(330.0, 282.0, 23.0, 23.0)),
                item(rect(363.0, 282.0, 23.0, 23.0)),
            ],
        }
    }

    fn shrunk(text_height: f32) -> DialogTemplate {
        shrink_to_text(&stock(), 3, &[1, 5, 6], text_height)
    }

    fn bounds(template: &DialogTemplate, item: usize) -> Bounds {
        template.items[item - 1].bounds
    }

    #[test]
    fn the_rules_constants() {
        assert_eq!((MIN_TEXT_HEIGHT, MARGIN), (48.0, 16.0));
    }

    #[test]
    fn one_line_shrinks_3003_to_441_by_115() {
        let one = shrunk(12.0);
        assert_eq!(one.bounds, rect(0.0, 0.0, 441.0, 115.0));
        assert_eq!(one.placement, Placement::CenterWindow);
        assert_eq!(bounds(&one, 1), rect(173.0, 83.0, 99.0, 25.0));
        assert_eq!(bounds(&one, 5), rect(330.0, 84.0, 23.0, 23.0));
        assert_eq!(bounds(&one, 6), rect(363.0, 84.0, 23.0, 23.0));
        assert_eq!(bounds(&one, 3), rect(11.0, 10.0, 417.0, 64.0));
        assert_eq!(bounds(&one, 2), bounds(&stock(), 2), "parked, untouched");
        assert_eq!(bounds(&one, 4), bounds(&stock(), 4), "parked, untouched");
    }

    #[test]
    fn a_text_under_48_high_shrinks_as_one_48_high_does() {
        assert_eq!(shrunk(12.0), shrunk(48.0));
        assert_eq!(shrunk(0.0), shrunk(48.0));
    }

    #[test]
    fn a_taller_text_keeps_its_height_plus_the_margin() {
        let tall = shrunk(100.0);
        assert_eq!(tall.bounds, rect(0.0, 0.0, 441.0, 167.0));
        assert_eq!(bounds(&tall, 3), rect(11.0, 10.0, 417.0, 116.0));
        assert_eq!(bounds(&tall, 1).min.y, 135.0);
    }

    #[test]
    fn a_text_246_high_leaves_the_geometry_as_it_is() {
        let even = shrunk(246.0);
        assert_eq!(even.bounds, stock().bounds);
        assert_eq!(even.items, stock().items);
    }

    #[test]
    fn a_text_just_short_of_the_box_grows_the_dialog() {
        let grown = shrunk(258.0);
        assert_eq!(grown.bounds, rect(0.0, 0.0, 441.0, 325.0));
        assert_eq!(bounds(&grown, 1), rect(173.0, 293.0, 99.0, 25.0));
        assert_eq!(bounds(&grown, 5).min.y, 294.0);
        assert_eq!(bounds(&grown, 3), rect(11.0, 10.0, 417.0, 274.0));
        assert_eq!(bounds(&grown, 2), bounds(&stock(), 2));
        assert_eq!(shrunk(261.0).bounds.height(), 328.0, "by at most 15");
    }

    #[test]
    fn a_text_that_fills_the_box_keeps_the_stock_size() {
        for h in [262.0, 400.0] {
            assert_eq!(shrunk(h), stock(), "{h}");
        }
    }

    #[test]
    fn no_text_item_leaves_it_unchanged() {
        let mut short = stock();
        short.items.truncate(2);
        assert_eq!(shrink_to_text(&short, 3, &[1], 12.0), short);
        assert_eq!(shrink_to_text(&stock(), 0, &[1], 12.0), stock());
    }

    #[test]
    fn a_moved_item_past_the_end_or_zero_is_ignored() {
        let one = shrink_to_text(&stock(), 3, &[1, 0, 9], 12.0);
        assert_eq!(one.bounds.height(), 115.0);
        assert_eq!(bounds(&one, 1).min.y, 83.0);
        assert_eq!(bounds(&one, 5), bounds(&stock(), 5), "not named");
    }

    #[test]
    fn a_shrink_keeps_the_top_left_corner() {
        let mut moved = stock();
        moved.bounds = moved.bounds.offset(Point::new(40.0, 30.0));
        let one = shrink_to_text(&moved, 3, &[1, 5, 6], 12.0);
        assert_eq!(one.bounds, rect(40.0, 30.0, 441.0, 115.0));
    }

    /// Stock `DLOG` 1016 and its `DITL`: 441 x 317, accept (1), refuse (2),
    /// the 415 x 267 text box (3), the parked items (4, 5, 7, 8), the one
    /// button (6) and the two scroll arrows (9, 10).
    fn stock_offer() -> DialogTemplate {
        let item = |(x, y, w, h)| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: false,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 441.0, 317.0),
            placement: Placement::CenterWindow,
            items: [
                (225.0, 285.0, 99.0, 25.0),
                (117.0, 285.0, 99.0, 25.0),
                (12.0, 9.0, 415.0, 267.0),
                (150.0, 323.0, 32.0, 32.0),
                (197.0, 326.0, 32.0, 32.0),
                (173.0, 285.0, 99.0, 25.0),
                (172.0, 337.0, 32.0, 32.0),
                (272.0, 345.0, 23.0, 23.0),
                (340.0, 287.0, 23.0, 23.0),
                (373.0, 287.0, 23.0, 23.0),
            ]
            .into_iter()
            .map(item)
            .collect(),
        }
    }

    #[test]
    fn a_texts_height_is_its_wrapped_lines_times_the_line_height() {
        let (font, size) = (Font::Geneva, 12.0);
        let line = MonoMetrics.line_height(font, size);
        let height = |text, width| wrapped_height(&MonoMetrics, font, size, text, width);
        let one = MonoMetrics.width(font, size, "xxxx");
        assert_eq!(height("xxxx", one), line);
        assert_eq!(height("xxxx xxxx", one), 2.0 * line, "at the box's width");
        assert_eq!(height("xxxx\rxxxx\rxxxx", 100.0), 3.0 * line);
        assert_eq!(height("", 100.0), 0.0);
    }

    fn placed(template: &DialogTemplate, text_height: f32) -> DialogTemplate {
        shrink_after_placing(template, 3, &[1, 2, 6], text_height)
    }

    #[test]
    fn the_windows_move_is_three_tenths_of_the_shrink() {
        assert_eq!(WINDOW_MOVE_FACTOR.to_bits(), 0x3FD3_3333_3333_3333);
    }

    #[test]
    fn each_height_places_stock_1016_as_the_original_does() {
        // text h, window (x, y, w, h), text box h, buttons' y.
        let rows = [
            (12.0, (292.0, 286.0, 441.0, 114.0), 64.0, 82.0),
            (48.0, (292.0, 286.0, 441.0, 114.0), 64.0, 82.0),
            (100.0, (292.0, 271.0, 441.0, 166.0), 116.0, 134.0),
            (251.0, (292.0, 226.0, 441.0, 317.0), 267.0, 285.0),
            (258.0, (292.0, 223.0, 441.0, 324.0), 274.0, 292.0),
            (266.0, (292.0, 221.0, 441.0, 332.0), 282.0, 300.0),
            (267.0, (292.0, 226.0, 441.0, 317.0), 267.0, 285.0),
            (400.0, (292.0, 226.0, 441.0, 317.0), 267.0, 285.0),
        ];
        let stock = stock_offer();
        for (h, (x, y, w, tall), text_box, buttons_y) in rows {
            let laid = placed(&stock, h);
            assert_eq!(laid.bounds, rect(x, y, w, tall), "{h}");
            assert_eq!(bounds(&laid, 3), rect(12.0, 9.0, 415.0, text_box), "{h}");
            for (number, left) in [(1, 225.0), (2, 117.0), (6, 173.0)] {
                assert_eq!(
                    bounds(&laid, number),
                    rect(left, buttons_y, 99.0, 25.0),
                    "{h}, item {number}"
                );
            }
            for number in [4, 5, 7, 8, 9, 10] {
                assert_eq!(bounds(&laid, number), bounds(&stock, number), "{h}");
            }
        }
    }

    #[test]
    fn the_move_truncates_the_sum_and_not_the_product() {
        let mut high = stock_offer();
        high.bounds = rect(50.0, 226.0, 441.0, 317.0);
        high.placement = Placement::Fixed;
        assert_eq!(placed(&high, 258.0).bounds.min.y, 223.0, "not 224");
    }

    #[test]
    fn the_result_stays_where_it_was_put() {
        assert_eq!(placed(&stock_offer(), 12.0).placement, Placement::Fixed);
    }

    #[test]
    fn it_is_placed_at_its_unshrunk_size() {
        let mut center = stock_offer();
        center.placement = Placement::Center;
        // Centred at 441 x 317, (291, 225), then down by 203 × 0.3.
        assert_eq!(
            placed(&center, 12.0).bounds,
            rect(291.0, 285.0, 441.0, 114.0)
        );
    }

    #[test]
    fn a_fixed_template_keeps_its_corner_across_and_moves_down() {
        let mut fixed = stock_offer();
        fixed.bounds = rect(40.0, 30.0, 441.0, 317.0);
        fixed.placement = Placement::Fixed;
        assert_eq!(placed(&fixed, 12.0).bounds, rect(40.0, 90.0, 441.0, 114.0));
    }

    #[test]
    fn a_template_with_no_text_item_stays_where_its_placement_puts_it() {
        let mut short = stock_offer();
        short.items.truncate(2);
        let laid = placed(&short, 12.0);
        assert_eq!(laid.bounds, rect(292.0, 226.0, 441.0, 317.0));
        assert_eq!(laid.items, short.items);
    }
}
