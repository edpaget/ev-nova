//! A number as the original writes it ([`comma_number`]), from
//! `nova_sim_story::text_tags`, which re-exports it and keeps the text
//! tags.

use crate::rulebook::RuleSource;

/// `n` grouped by threes with commas: "1,234,567".
fn with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() * 4 / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// `n` as the original's `_StrcatCommaNumber` (@0x8820) writes it: plain
/// below 1000, and "1,234" below a million. From a million,
/// [`RuleKey::PayMillions`](crate::RuleKey::PayMillions) chooses: by the
/// engine, in millions to two places, truncated, "1.23M"
/// (@0x8990-0x8aa3); by the other reading, with its commas, "1,234,567".
#[must_use]
pub fn comma_number(n: i64, millions: RuleSource) -> String {
    match n {
        ..1000 => n.to_string(),
        1_000_000.. if millions == RuleSource::Engine => {
            format!("{}.{:02}M", n / 1_000_000, n % 1_000_000 / 10_000)
        }
        _ => with_commas(n.unsigned_abs()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_below_1000_is_plain() {
        assert_eq!(comma_number(0, RuleSource::Engine), "0");
        assert_eq!(comma_number(999, RuleSource::Engine), "999");
        assert_eq!(comma_number(-5, RuleSource::Engine), "-5");
        assert_eq!(comma_number(-1234, RuleSource::Engine), "-1234");
    }

    #[test]
    fn a_number_in_the_thousands_takes_a_comma() {
        assert_eq!(comma_number(1000, RuleSource::Engine), "1,000");
        assert_eq!(comma_number(1005, RuleSource::Engine), "1,005");
        assert_eq!(comma_number(999_999, RuleSource::Engine), "999,999");
    }

    #[test]
    fn by_the_engine_a_million_or_more_reads_in_millions_to_two_places_truncated() {
        assert_eq!(comma_number(1_000_000, RuleSource::Engine), "1.00M");
        assert_eq!(comma_number(1_050_000, RuleSource::Engine), "1.05M");
        assert_eq!(comma_number(1_234_567, RuleSource::Engine), "1.23M");
        assert_eq!(comma_number(1_239_999, RuleSource::Engine), "1.23M");
        assert_eq!(
            comma_number(12_345_678_901, RuleSource::Engine),
            "12345.67M"
        );
    }

    #[test]
    fn by_the_other_reading_a_million_or_more_takes_its_commas() {
        assert_eq!(comma_number(1_000_000, RuleSource::Bible), "1,000,000");
        assert_eq!(comma_number(1_234_567, RuleSource::Bible), "1,234,567");
        assert_eq!(
            comma_number(12_345_678_901, RuleSource::Bible),
            "12,345,678,901"
        );
        assert_eq!(comma_number(999_999, RuleSource::Bible), "999,999");
        assert_eq!(comma_number(999, RuleSource::Bible), "999");
    }
}
