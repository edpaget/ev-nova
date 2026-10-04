//! Test helpers for the [`TextMetrics`] port: a monospaced mock and the
//! contract every adapter must meet.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests: `nova-render` checks its glyphon adapter with
//! [`check_contract`].

use super::{LINE_HEIGHT, TextMetrics};
use crate::font::Font;

/// Monospaced metrics: every character is half the size wide in Geneva and
/// six tenths of it in Charcoal, and lines are [`LINE_HEIGHT`] times the
/// size apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonoMetrics;

impl MonoMetrics {
    /// One character's width in `font` at `size`.
    #[must_use]
    pub fn advance(font: Font, size: f32) -> f32 {
        match font {
            Font::Geneva => 0.5 * size,
            Font::Charcoal => 0.6 * size,
        }
    }
}

impl TextMetrics for MonoMetrics {
    fn width(&self, font: Font, size: f32, text: &str) -> f32 {
        text.chars().count() as f32 * Self::advance(font, size)
    }

    fn line_height(&self, _font: Font, size: f32) -> f32 {
        LINE_HEIGHT * size
    }
}

/// The size the contract measures at.
const SIZE: f32 = 10.0;

/// A text whose width tells two different faces apart.
const SAMPLE: &str = "Kestrel MMM iii";

/// Checks the port's contract, which [`MonoMetrics`] encodes, against
/// `metrics`, in both fonts:
///
/// - the empty string is 0 wide;
/// - width grows strictly as characters are added (`""` < `"a"` < `"aa"` <
///   `"aaa"`);
/// - the line height is [`LINE_HEIGHT`] times the size, and the same on
///   every call;
/// - width and line height at twice the size are twice those at the size
///   (within 1%), so text scales with the UI;
/// - Charcoal and Geneva measure a sample text differently, so the font is
///   honoured (given faces that differ).
///
/// The error says which clause failed.
pub fn check_contract(metrics: &impl TextMetrics) -> Result<(), String> {
    for font in [Font::Geneva, Font::Charcoal] {
        let empty = metrics.width(font, SIZE, "");
        if empty != 0.0 {
            return Err(format!("{font:?}: the empty string is {empty} wide, not 0"));
        }
        let widths = ["", "a", "aa", "aaa"].map(|text| metrics.width(font, SIZE, text));
        if !widths.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(format!(
                "{font:?}: widths of \"\" to \"aaa\" do not grow strictly: {widths:?}"
            ));
        }
        let line = metrics.line_height(font, SIZE);
        if !close(line, LINE_HEIGHT * SIZE, 1e-4) {
            return Err(format!(
                "{font:?}: the line height at {SIZE} is {line}, not {}",
                LINE_HEIGHT * SIZE
            ));
        }
        let again = metrics.line_height(font, SIZE);
        if again.to_bits() != line.to_bits() {
            return Err(format!(
                "{font:?}: the line height changed between calls: {line}, then {again}"
            ));
        }
        let (width, doubled) = (
            metrics.width(font, SIZE, SAMPLE),
            metrics.width(font, 2.0 * SIZE, SAMPLE),
        );
        if !close(doubled, 2.0 * width, 0.01) {
            return Err(format!(
                "{font:?}: the width at twice the size is {doubled}, not twice {width}"
            ));
        }
        let doubled = metrics.line_height(font, 2.0 * SIZE);
        if !close(doubled, 2.0 * line, 0.01) {
            return Err(format!(
                "{font:?}: the line height at twice the size is {doubled}, not twice {line}"
            ));
        }
    }
    let charcoal = metrics.width(Font::Charcoal, SIZE, SAMPLE);
    let geneva = metrics.width(Font::Geneva, SIZE, SAMPLE);
    if charcoal.to_bits() == geneva.to_bits() {
        return Err(format!(
            "Charcoal and Geneva measure {SAMPLE:?} the same: {charcoal}"
        ));
    }
    Ok(())
}

/// Whether `value` is within `tolerance` (a fraction) of `expected`; never
/// for NaN.
fn close(value: f32, expected: f32, tolerance: f32) -> bool {
    (value - expected).abs() <= tolerance * expected.abs()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn mono_metrics_is_half_the_size_a_character_in_geneva() {
        assert_eq!(MonoMetrics.width(Font::Geneva, 10.0, "abc"), 15.0);
        assert_eq!(MonoMetrics.width(Font::Geneva, 10.0, "äö"), 10.0);
        assert_eq!(MonoMetrics.width(Font::Charcoal, 10.0, "abc"), 18.0);
        assert_eq!(MonoMetrics.width(Font::Geneva, 10.0, ""), 0.0);
        assert_eq!(MonoMetrics.line_height(Font::Charcoal, 10.0), 12.0);
        assert_eq!(MonoMetrics::advance(Font::Charcoal, 20.0), 12.0);
    }

    #[test]
    fn mono_metrics_meets_the_contract() {
        assert_eq!(check_contract(&MonoMetrics), Ok(()));
    }

    /// [`MonoMetrics`] with one thing changed.
    struct Broken<W, L> {
        width: W,
        line_height: L,
    }

    impl<W, L> TextMetrics for Broken<W, L>
    where
        W: Fn(Font, f32, &str) -> f32,
        L: Fn(Font, f32) -> f32,
    {
        fn width(&self, font: Font, size: f32, text: &str) -> f32 {
            (self.width)(font, size, text)
        }

        fn line_height(&self, font: Font, size: f32) -> f32 {
            (self.line_height)(font, size)
        }
    }

    fn mono_width(font: Font, size: f32, text: &str) -> f32 {
        MonoMetrics.width(font, size, text)
    }

    fn mono_line(font: Font, size: f32) -> f32 {
        MonoMetrics.line_height(font, size)
    }

    fn broken_width(width: impl Fn(Font, f32, &str) -> f32) -> Result<(), String> {
        check_contract(&Broken {
            width,
            line_height: mono_line,
        })
    }

    fn broken_line(line_height: impl Fn(Font, f32) -> f32) -> Result<(), String> {
        check_contract(&Broken {
            width: mono_width,
            line_height,
        })
    }

    #[test]
    fn a_non_zero_empty_width_fails() {
        let err = broken_width(|font, size, text| mono_width(font, size, text) + 1.0);
        assert_eq!(
            err,
            Err("Geneva: the empty string is 1 wide, not 0".to_owned())
        );
        let err = broken_width(|font, size, text| match (font, text) {
            (Font::Charcoal, "") => f32::NAN,
            _ => mono_width(font, size, text),
        });
        assert_eq!(
            err,
            Err("Charcoal: the empty string is NaN wide, not 0".to_owned())
        );
    }

    #[test]
    fn a_width_that_does_not_grow_strictly_fails() {
        let capped = broken_width(|font, size, text| mono_width(font, size, text).min(10.0));
        assert_eq!(
            capped,
            Err(
                "Geneva: widths of \"\" to \"aaa\" do not grow strictly: [0.0, 5.0, 10.0, 10.0]"
                    .to_owned()
            )
        );
        let zero_a = broken_width(|font, size, text| match text {
            "a" => 0.0,
            _ => mono_width(font, size, text),
        });
        assert!(zero_a.is_err(), "{zero_a:?}");
        let shrinking = broken_width(|font, size, text| match (font, text) {
            (Font::Charcoal, "aa") => 1.0,
            _ => mono_width(font, size, text),
        });
        assert!(
            shrinking.is_err_and(|err| err.starts_with("Charcoal: widths")),
            "only Charcoal shrinks"
        );
    }

    #[test]
    fn a_wrong_line_height_fails() {
        let err = broken_line(|_, size| size);
        assert_eq!(
            err,
            Err("Geneva: the line height at 10 is 10, not 12".to_owned())
        );
        assert!(broken_line(|_, size| 1.201 * size).is_err());
        assert!(broken_line(|_, _| f32::NAN).is_err());
        assert_eq!(broken_line(|_, size| 1.200_01 * size), Ok(()));
    }

    #[test]
    fn a_line_height_that_changes_between_calls_fails() {
        let calls = Cell::new(0_u32);
        let err = broken_line(|_, size| {
            calls.set(calls.get() + 1);
            LINE_HEIGHT * size + calls.get() as f32 * 1e-6
        });
        assert!(
            err.as_ref()
                .is_err_and(|err| err.starts_with("Geneva: the line height changed")),
            "{err:?}"
        );
    }

    #[test]
    fn a_width_not_proportional_to_the_size_fails() {
        let err = broken_width(|font, size, text| match text {
            SAMPLE => mono_width(font, SIZE, text),
            _ => mono_width(font, size, text),
        });
        assert_eq!(
            err,
            Err("Geneva: the width at twice the size is 75, not twice 75".to_owned())
        );
        let near = broken_width(|font, size, text| {
            let width = mono_width(font, size, text);
            if size > SIZE { width * 1.009 } else { width }
        });
        assert_eq!(near, Ok(()), "within 1%");
        let off = broken_width(|font, size, text| {
            let width = mono_width(font, size, text);
            if size > SIZE { width * 1.011 } else { width }
        });
        assert!(off.is_err(), "past 1%");
    }

    #[test]
    fn a_line_height_not_proportional_to_the_size_fails() {
        let err = broken_line(|_, _| 12.0);
        assert_eq!(
            err,
            Err("Geneva: the line height at twice the size is 12, not twice 12".to_owned())
        );
    }

    #[test]
    fn fonts_measured_the_same_fail() {
        let err = broken_width(|_, size, text| mono_width(Font::Geneva, size, text));
        assert_eq!(
            err,
            Err("Charcoal and Geneva measure \"Kestrel MMM iii\" the same: 75".to_owned())
        );
    }
}
