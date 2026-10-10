//! Amounts of credits as the original's shop dialogs draw them:
//! `_DrawCommaNumber` (@0x4895), a space (0xd97a0) and "cr".

/// `STR#` 2002 #34, after each amount.
pub const CR: &str = "cr";
/// `STR#` 2002 #217, the cash's label.
pub const YOU_HAVE: &str = "You Have:";

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
    fn credits_text_puts_a_space_and_cr_after_the_amount() {
        assert_eq!(credits_text(25_000), "25,000 cr");
        assert_eq!(credits_text(0), "0 cr");
        assert_eq!(credits_text(-250), "-250 cr");
        assert_eq!((CR, YOU_HAVE), ("cr", "You Have:"));
    }
}
