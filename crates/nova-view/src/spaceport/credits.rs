//! Amounts of credits as the original's shop dialogs draw them:
//! `_DrawCommaNumber` (@0x4895), a space (0xd97a0) and "cr", and the
//! column the amounts start in; and numbers of tons with their unit.

use nova_sim::{ShownTons, TonUnit};

use super::grid::{self, TEXT_COLOR, text};
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};

/// `STR#` 2002 #34, after each amount.
pub const CR: &str = "cr";
/// `STR#` 2002 #217, the cash's label.
pub const YOU_HAVE: &str = "You Have:";
/// How far each amount starts right of its info box's left: `MoveTo`
/// h + 0x46 (`_ShipyardDialogUpdate` @0x5931c-0x59333,
/// `_OutfitDialogUpdate` @0x57b6e-0x57b85).
pub const AMOUNT_COLUMN: f32 = 70.0;

/// Draws an info box's amount rows, each `(baseline, label, amount)`: the
/// label at `info`'s left and the amount [`AMOUNT_COLUMN`] right of it, in
/// white, on a line whose baseline is `baseline` below `info`'s top, as the
/// shop dialogs' updates do.
pub fn draw_amount_rows<'a>(
    list: &mut DrawList,
    info: Bounds,
    rows: impl IntoIterator<Item = (f32, &'a str, String)>,
) {
    for (baseline, label, amount) in rows {
        let top = grid::baseline_top(info.min.y + baseline);
        text(list, label, Point::new(info.min.x, top), TEXT_COLOR);
        text(
            list,
            &amount,
            Point::new(info.min.x + AMOUNT_COLUMN, top),
            TEXT_COLOR,
        );
    }
}

/// `n` as `_DrawCommaNumber` (@0x4895) draws it: up to 999, negatives
/// included, plain digits (@0x48aa-0x48c7); up to 999,999, thousands, a
/// comma and three digits (@0x48cc-0x4948); otherwise millions, a point,
/// two digits of the rest truncated, and "M" (@0x494d-0x49d2).
#[must_use]
pub fn comma_number(n: i64) -> String {
    if n <= 999 {
        n.to_string()
    } else if n <= 999_999 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        format!("{}.{:02}M", n / 1_000_000, n % 1_000_000 / 10_000)
    }
}

/// `n` credits as the shop dialogs draw them: [`comma_number`]'s text, a
/// space and [`CR`].
#[must_use]
pub fn credits_text(n: i64) -> String {
    format!("{} {CR}", comma_number(n))
}

/// `STR#` 2002 #1 (`_tonString`).
pub const TON: &str = "ton";
/// `STR#` 2002 #2 (`_tonsString`).
pub const TONS: &str = "tons";

/// The words for `unit`; the core picks the unit.
fn ton_word(unit: TonUnit) -> &'static str {
    match unit {
        TonUnit::Ton => TON,
        TonUnit::Tons => TONS,
    }
}

/// `shown`'s number and the words for its unit, in plain digits, as the
/// Shipyard Info panel's `NumToString` draws them.
pub(crate) fn tons_text(shown: ShownTons) -> String {
    format!("{} {}", shown.tons, ton_word(shown.unit))
}

/// `shown` as the outfitter's info box draws it: `_DrawCommaNumber`'s
/// text, a space and the unit (@0x57cf6-0x57d19, @0x57d7a-0x57d9d).
pub(crate) fn comma_tons_text(shown: ShownTons) -> String {
    format!("{} {}", comma_number(shown.tons), ton_word(shown.unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_to_999_and_every_negative_draw_as_plain_digits() {
        for (n, drawn) in [
            (0, "0"),
            (7, "7"),
            (999, "999"),
            (-5, "-5"),
            (-1500, "-1500"),
            (-30_000, "-30000"),
            (-2_000_000, "-2000000"),
        ] {
            assert_eq!(comma_number(n), drawn, "{n}");
        }
    }

    #[test]
    fn thousands_draw_with_a_comma_and_three_digits() {
        for (n, drawn) in [
            (1000, "1,000"),
            (1005, "1,005"),
            (1050, "1,050"),
            (12_345, "12,345"),
            (999_999, "999,999"),
        ] {
            assert_eq!(comma_number(n), drawn, "{n}");
        }
    }

    #[test]
    fn a_million_or_more_draws_as_millions_two_truncated_digits_and_m() {
        for (n, drawn) in [
            (1_000_000, "1.00M"),
            (1_050_000, "1.05M"),
            (1_234_567, "1.23M"),
            (1_999_999, "1.99M"),
            (2_500_000, "2.50M"),
            (12_345_678, "12.34M"),
            (3_000_000_000, "3000.00M"),
        ] {
            assert_eq!(comma_number(n), drawn, "{n}");
        }
    }

    #[test]
    fn comma_tons_text_separates_and_tons_text_does_not() {
        for (tons, text) in [
            (0, "0 tons"),
            (1, "1 ton"),
            (999, "999 tons"),
            (1_000, "1,000 tons"),
            (1_234_567, "1.23M tons"),
        ] {
            assert_eq!(comma_tons_text(ShownTons::of(tons)), text, "{tons}");
        }
        assert_eq!(tons_text(ShownTons::of(1_000)), "1000 tons");
        assert_eq!(tons_text(ShownTons::of(1)), "1 ton");
    }

    #[test]
    fn each_amount_row_draws_its_label_at_the_left_and_its_amount_70_right() {
        use crate::draw::{DrawCommand, DrawList};
        use crate::geometry::{Bounds, Point};
        use crate::spaceport::grid::{TEXT_COLOR, TEXT_FONT, TEXT_SIZE};

        let info = Bounds::at(Point::new(100.0, 200.0), 150.0, 90.0);
        let mut list = DrawList::new();
        draw_amount_rows(
            &mut list,
            info,
            [
                (12.0, "Hiring Price:", "970 cr".to_owned()),
                (36.0, "You Have:", "25,000 cr".to_owned()),
            ],
        );
        let drawn: Vec<_> = list
            .iter()
            .map(|command| match command {
                DrawCommand::Text {
                    text,
                    font,
                    origin,
                    size,
                    wrap_width,
                    color,
                } => {
                    assert_eq!(
                        (*font, *size, *wrap_width, *color),
                        (TEXT_FONT, TEXT_SIZE, None, TEXT_COLOR)
                    );
                    (text.clone(), *origin)
                }
                other => panic!("not text: {other:?}"),
            })
            .collect();
        // Tops are the baselines less the text size, 10.
        assert_eq!(
            drawn,
            [
                ("Hiring Price:".to_owned(), Point::new(100.0, 202.0)),
                ("970 cr".to_owned(), Point::new(170.0, 202.0)),
                ("You Have:".to_owned(), Point::new(100.0, 226.0)),
                ("25,000 cr".to_owned(), Point::new(170.0, 226.0)),
            ]
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn the_amount_column_is_70() {
        assert_eq!(AMOUNT_COLUMN, 70.0);
    }

    #[test]
    fn credits_text_puts_a_space_and_cr_after_the_amount() {
        assert_eq!(credits_text(25_000), "25,000 cr");
        assert_eq!(credits_text(0), "0 cr");
        assert_eq!(credits_text(-250), "-250 cr");
        assert_eq!((CR, YOU_HAVE), ("cr", "You Have:"));
    }
}
