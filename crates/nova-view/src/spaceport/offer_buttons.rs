//! The offer dialog's buttons widened to their labels, as
//! `_OfferOneMission` widens them (@0xa2a29-0xa2beb) after the shrink, in
//! `DLOG` 1016 and 1020 alike. It changes only x.
//!
//! The width is `w = max(StringWidth(accept), StringWidth(refuse)) + `
//! [`LABEL_PADDING`] (@0xa2a50-0xa2a81), at least [`LEAST_WIDTH`]
//! (`cmovle` @0xa2a84-0xa2a8d), so every offer's buttons are wider than the
//! items' 99. The labels are measured in the system font at size 12
//! (`TextFont(0)` @0xa2a29, `TextSize(0xc)` @0xa2a35) whatever the button
//! style draws them in. Then:
//!
//! - Item 1's left edge stays and its right is left + `w` (@0xa2aa7-0xa2ada).
//! - Item 2's right edge stays and its left is right − `w`
//!   (@0xa2b12-0xa2b46).
//! - Item 6 widens about `trunc((left + right) / 2)`, by `w / 2` truncated
//!   toward zero each way (`InsetRect` @0xa2b7e-0xa2bbd, `SetDialogItem`
//!   @0xa2beb), so an odd `w` leaves it `w − 1` wide.
//!
//! The original measures a one-button offer's refuse string too, its
//! `RefuseButton` or `STR#` 150 #51 (@0xa29d7-0xa2a14, @0xa2a55).
//! [`OfferLabels`] holds none for an offer that can't be refused, so the
//! accept label alone sets `w` here. No stock mission shows the difference.

use nova_sim::mission::OfferLabels;

use super::offer::{ACCEPT_ITEM, REFUSE_ITEM, SINGLE_ITEM};
use crate::font::Font;
use crate::geometry::Bounds;
use crate::text::TextMetrics;
use crate::ui::dialog::DialogTemplate;

/// The font the labels are measured in: the engine's system font, as
/// `TextFont(0)` asks for (@0xa2a29).
pub(super) const LABEL_FONT: Font = Font::Charcoal;

/// The size the labels are measured at (`TextSize(0xc)` @0xa2a35).
pub(super) const LABEL_SIZE: f32 = 12.0;

/// The room a button leaves around its label (@0xa2a81).
pub(super) const LABEL_PADDING: f32 = 40.0;

/// The least width of a widened button (@0xa2a84-0xa2a8d).
pub(super) const LEAST_WIDTH: f32 = 100.0;

/// Widens `template`'s accept, refuse and one button to `labels`, as the
/// module docs say. An item the template doesn't have is skipped.
pub(super) fn widen_to_labels(
    template: &mut DialogTemplate,
    labels: &OfferLabels,
    metrics: &dyn TextMetrics,
) {
    let measure = |label: &str| metrics.width(LABEL_FONT, LABEL_SIZE, label);
    let wider = labels
        .refuse
        .as_deref()
        .map_or(0.0, measure)
        .max(measure(&labels.accept));
    let width = (wider + LABEL_PADDING).max(LEAST_WIDTH);
    if let Some(accept) = bounds_of(template, ACCEPT_ITEM) {
        accept.max.x = accept.min.x + width;
    }
    if let Some(refuse) = bounds_of(template, REFUSE_ITEM) {
        refuse.min.x = refuse.max.x - width;
    }
    if let Some(single) = bounds_of(template, SINGLE_ITEM) {
        let centre = f32::midpoint(single.min.x, single.max.x).trunc();
        let half = (width / 2.0).trunc();
        single.min.x = centre - half;
        single.max.x = centre + half;
    }
}

/// Item `number`'s bounds (from 1), if `template` has it.
fn bounds_of(template: &mut DialogTemplate, number: usize) -> Option<&mut Bounds> {
    template
        .items
        .get_mut(number - 1)
        .map(|item| &mut item.bounds)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    /// Metrics whose every character is 1 wide, for whole widths.
    struct OneWide;

    impl TextMetrics for OneWide {
        fn width(&self, _font: Font, _size: f32, text: &str) -> f32 {
            text.chars().count() as f32
        }

        fn line_height(&self, _font: Font, _size: f32) -> f32 {
            12.0
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn template(buttons: [(f32, f32); 3]) -> DialogTemplate {
        let mut items: Vec<ItemTemplate> = (0..6)
            .map(|_| ItemTemplate {
                bounds: rect(12.0, 9.0, 415.0, 267.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        for (number, (x, y)) in [ACCEPT_ITEM, REFUSE_ITEM, SINGLE_ITEM]
            .into_iter()
            .zip(buttons)
        {
            items[number - 1].bounds = rect(x, y, 99.0, 25.0);
        }
        DialogTemplate {
            bounds: rect(0.0, 0.0, 441.0, 317.0),
            placement: Placement::CenterWindow,
            items,
        }
    }

    /// `DITL` 1016's buttons.
    fn stock_1016() -> DialogTemplate {
        template([(225.0, 285.0), (117.0, 285.0), (173.0, 285.0)])
    }

    /// `DITL` 1020's buttons.
    fn stock_1020() -> DialogTemplate {
        template([(221.0, 213.0), (115.0, 213.0), (170.0, 213.0)])
    }

    fn labels(accept: &str, refuse: Option<&str>) -> OfferLabels {
        OfferLabels {
            accept: accept.to_owned(),
            refuse: refuse.map(str::to_owned),
        }
    }

    fn widened(mut template: DialogTemplate, labels: &OfferLabels) -> DialogTemplate {
        widen_to_labels(&mut template, labels, &OneWide);
        template
    }

    fn item(template: &DialogTemplate, number: usize) -> Bounds {
        template.items[number - 1].bounds
    }

    /// Labels `accept` and `refuse` characters long.
    fn sized(accept: usize, refuse: usize) -> OfferLabels {
        labels(&"x".repeat(accept), Some(&"x".repeat(refuse)))
    }

    #[test]
    fn the_named_values() {
        assert_eq!(LABEL_FONT, Font::Charcoal);
        assert_eq!(
            (LABEL_SIZE, LABEL_PADDING, LEAST_WIDTH),
            (12.0, 40.0, 100.0)
        );
    }

    #[test]
    fn short_labels_take_the_floor_in_both_variants() {
        let short = labels("Yes", Some("No"));
        let plain = widened(stock_1016(), &short);
        assert_eq!(item(&plain, ACCEPT_ITEM), rect(225.0, 285.0, 100.0, 25.0));
        assert_eq!(item(&plain, REFUSE_ITEM), rect(116.0, 285.0, 100.0, 25.0));
        assert_eq!(item(&plain, SINGLE_ITEM), rect(172.0, 285.0, 100.0, 25.0));
        let picture = widened(stock_1020(), &short);
        assert_eq!(item(&picture, ACCEPT_ITEM), rect(221.0, 213.0, 100.0, 25.0));
        assert_eq!(item(&picture, REFUSE_ITEM), rect(114.0, 213.0, 100.0, 25.0));
        assert_eq!(item(&picture, SINGLE_ITEM), rect(169.0, 213.0, 100.0, 25.0));
    }

    #[test]
    fn the_wider_label_sets_both_buttons_whichever_it_is() {
        for wide in [sized(80, 10), sized(10, 80)] {
            let laid = widened(stock_1016(), &wide);
            assert_eq!(item(&laid, ACCEPT_ITEM), rect(225.0, 285.0, 120.0, 25.0));
            assert_eq!(item(&laid, REFUSE_ITEM), rect(96.0, 285.0, 120.0, 25.0));
        }
    }

    #[test]
    fn a_width_of_99_becomes_100_and_101_stays() {
        let width = |long| item(&widened(stock_1016(), &sized(long, 1)), ACCEPT_ITEM).width();
        assert_eq!(width(59), 100.0);
        assert_eq!(width(60), 100.0);
        assert_eq!(width(61), 101.0);
    }

    #[test]
    fn the_one_button_widens_about_its_centre_and_an_odd_width_loses_a_pixel() {
        let even = widened(stock_1016(), &sized(80, 1));
        assert_eq!(item(&even, SINGLE_ITEM), rect(162.0, 285.0, 120.0, 25.0));
        let odd = widened(stock_1016(), &sized(81, 1));
        assert_eq!(item(&odd, SINGLE_ITEM), rect(162.0, 285.0, 120.0, 25.0));
        let next = widened(stock_1016(), &sized(82, 1));
        assert_eq!(item(&next, SINGLE_ITEM), rect(161.0, 285.0, 122.0, 25.0));
    }

    #[test]
    fn an_offer_that_cannot_be_refused_measures_its_accept_alone() {
        let laid = widened(stock_1016(), &labels(&"x".repeat(70), None));
        assert_eq!(item(&laid, SINGLE_ITEM), rect(167.0, 285.0, 110.0, 25.0));
    }

    #[test]
    fn it_measures_in_the_system_font_at_12() {
        let mut laid = stock_1016();
        let long = "I need more time";
        widen_to_labels(&mut laid, &labels(long, Some("No")), &MonoMetrics);
        assert_eq!(
            item(&laid, ACCEPT_ITEM).width(),
            MonoMetrics.width(Font::Charcoal, 12.0, long) + 40.0
        );
    }

    #[test]
    fn a_template_without_an_item_widens_the_others() {
        let mut short = stock_1016();
        short.items.truncate(2);
        let laid = widened(short, &labels("Yes", Some("No")));
        assert_eq!(item(&laid, ACCEPT_ITEM).width(), 100.0);
        assert_eq!(item(&laid, REFUSE_ITEM).width(), 100.0);
        assert_eq!(laid.items.len(), 2);
        let mut none = stock_1016();
        none.items.clear();
        assert_eq!(widened(none.clone(), &labels("Yes", None)), none);
    }

    #[test]
    fn nothing_but_the_buttons_x_changes() {
        let stock = stock_1016();
        let laid = widened(stock.clone(), &sized(90, 3));
        assert_eq!(laid.bounds, stock.bounds);
        for number in 1..=6 {
            let (before, after) = (item(&stock, number), item(&laid, number));
            assert_eq!((after.min.y, after.max.y), (before.min.y, before.max.y));
        }
        assert_eq!(item(&laid, 3), item(&stock, 3));
    }
}
