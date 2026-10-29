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
//! rule (@0xa2548-0xa2640), but after `_CenterWindow` has placed it, so
//! its window is also moved down by `trunc(0.3 × d)` (the double at
//! 0xdd8d0), and it moves 1016's own items.

use crate::geometry::Point;

use super::dialog::DialogTemplate;

/// The least text height the shrink leaves room for: a shorter text keeps
/// a box this tall plus [`MARGIN`] (`cmovge` on 0x30 @0x5aaed).
pub const MIN_TEXT_HEIGHT: f32 = 48.0;

/// The room the shrink leaves under the text in its box (@0x5aaf6).
pub const MARGIN: f32 = 16.0;

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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Bounds;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

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
}
