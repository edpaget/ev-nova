//! Text measurement: the [`TextMetrics`] port, the shared line-height rule
//! and word wrap.
//!
//! Everything is in logical units. The port has no scale: the renderer
//! multiplies a text run's size, origin and wrap width by the UI scale
//! when it draws, so widths and line heights that are proportional to the
//! size are all that "text scales with the rest of the UI" needs.
//!
//! The `fixture` feature (and this crate's tests) exposes `fixture`: a
//! monospaced mock and the port's contract, which every adapter must meet.

use std::rc::Rc;

use crate::font::Font;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// Line height as a multiple of the font size. `nova-render`'s batcher
/// spaces drawn lines by this same constant, so layout and drawing agree.
pub const LINE_HEIGHT: f32 = 1.2;

/// Measures text as the renderer will draw it.
pub trait TextMetrics {
    /// The advance width of `text` set on one line (no wrapping) in `font`
    /// at `size`, in logical units. The empty string is 0 wide.
    fn width(&self, font: Font, size: f32, text: &str) -> f32;
    /// The distance between the tops of successive lines in `font` at
    /// `size`, in logical units: [`LINE_HEIGHT`] times `size`.
    fn line_height(&self, font: Font, size: f32) -> f32;
}

/// Borrowed metrics are metrics.
impl<T: TextMetrics + ?Sized> TextMetrics for &T {
    fn width(&self, font: Font, size: f32, text: &str) -> f32 {
        (**self).width(font, size, text)
    }

    fn line_height(&self, font: Font, size: f32) -> f32 {
        (**self).line_height(font, size)
    }
}

/// Shared metrics are metrics, so every screen can measure with the one
/// the app was given.
impl<T: TextMetrics + ?Sized> TextMetrics for Rc<T> {
    fn width(&self, font: Font, size: f32, text: &str) -> f32 {
        (**self).width(font, size, text)
    }

    fn line_height(&self, font: Font, size: f32) -> f32 {
        (**self).line_height(font, size)
    }
}

/// `text` broken into lines no wider than `width`, measured by `metrics`
/// in `font` at `size`.
///
/// - Lines break greedily at spaces; each whole candidate line is measured,
///   so kerning counts.
/// - `\r` (as the Mac stores text), `\n` and `\r\n` are hard breaks, and a
///   blank line stays a blank line.
/// - A word wider than `width` breaks between characters.
/// - The spaces at a wrap point are dropped.
/// - Every line holds at least one character, so a width that is not a
///   positive, finite number still terminates: one character per line.
///
/// Empty text has no lines.
#[must_use]
pub fn wrap(
    metrics: &impl TextMetrics,
    font: Font,
    size: f32,
    text: &str,
    width: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    if text.is_empty() {
        return lines;
    }
    let fits = |line: &str| metrics.width(font, size, line) <= width;
    for paragraph in text.split("\r\n").flat_map(|part| part.split(['\r', '\n'])) {
        let before = lines.len();
        wrap_paragraph(paragraph, &fits, &mut lines);
        if lines.len() == before {
            lines.push(String::new());
        }
    }
    lines
}

/// Wraps one paragraph (no hard breaks) into `lines`.
fn wrap_paragraph(paragraph: &str, fits: &impl Fn(&str) -> bool, lines: &mut Vec<String>) {
    // The line being filled; `None` until something is put on it.
    let mut line: Option<String> = None;
    for word in paragraph.split(' ') {
        let candidate = match &line {
            None => word.to_owned(),
            Some(line) => format!("{line} {word}"),
        };
        if fits(&candidate) {
            line = Some(candidate);
            continue;
        }
        if let Some(full) = line.take() {
            lines.push(full.trim_end_matches(' ').to_owned());
        }
        if word.is_empty() {
            // A space at the wrap point.
            continue;
        }
        let mut rest = word;
        while !fits(rest) && rest.chars().nth(1).is_some() {
            let at = longest_fitting_prefix(rest, fits);
            lines.push(rest[..at].to_owned());
            rest = &rest[at..];
        }
        line = Some(rest.to_owned());
    }
    if let Some(last) = line {
        lines.push(last);
    }
}

/// The byte length of the longest prefix of `word` that fits, and at least
/// its first character.
fn longest_fitting_prefix(word: &str, fits: &impl Fn(&str) -> bool) -> usize {
    let mut ends = word.char_indices().map(|(at, c)| at + c.len_utf8());
    let first = ends.next().unwrap_or(word.len());
    ends.take_while(|&end| fits(&word[..end]))
        .last()
        .unwrap_or(first)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::fixture::MonoMetrics;
    use super::*;

    /// Geneva at size 10 in [`MonoMetrics`]: 5 units a character.
    fn geneva(text: &str, width: f32) -> Vec<String> {
        wrap(&MonoMetrics, Font::Geneva, 10.0, text, width)
    }

    #[test]
    fn a_line_that_fits_exactly_is_one_line() {
        assert_eq!(geneva("abcd", 20.0), ["abcd"]);
        assert_eq!(geneva("ab cd", 25.0), ["ab cd"]);
    }

    #[test]
    fn one_character_over_wraps_at_the_space() {
        assert_eq!(geneva("ab cd", 24.9), ["ab", "cd"]);
    }

    #[test]
    fn several_words_fill_each_line_greedily() {
        assert_eq!(
            geneva("the quick brown fox jumps", 50.0),
            ["the quick", "brown fox", "jumps"]
        );
    }

    #[test]
    fn mac_unix_and_dos_breaks_are_hard_breaks() {
        assert_eq!(geneva("a\rb\nc\r\nd", 100.0), ["a", "b", "c", "d"]);
    }

    #[test]
    fn a_blank_line_stays_blank() {
        assert_eq!(geneva("a\r\rb", 100.0), ["a", "", "b"]);
        assert_eq!(geneva("a\n\r\nb", 100.0), ["a", "", "b"]);
        assert_eq!(geneva("\r", 100.0), ["", ""]);
    }

    #[test]
    fn a_word_wider_than_the_box_breaks_between_characters() {
        assert_eq!(geneva("abcdefghij", 20.0), ["abcd", "efgh", "ij"]);
        assert_eq!(
            geneva("x abcdefghij y", 20.0),
            ["x", "abcd", "efgh", "ij y"]
        );
    }

    #[test]
    fn breaking_a_word_keeps_multi_byte_characters_whole() {
        assert_eq!(geneva("äöüß", 10.0), ["äö", "üß"]);
    }

    #[test]
    fn spaces_at_a_wrap_point_are_dropped() {
        assert_eq!(geneva("abc   def", 20.0), ["abc", "def"]);
        assert_eq!(geneva("abc def", 15.0), ["abc", "def"]);
    }

    #[test]
    fn spaces_inside_a_line_are_kept() {
        assert_eq!(geneva("a  b", 100.0), ["a  b"]);
        assert_eq!(geneva("  a", 100.0), ["  a"]);
        assert_eq!(geneva("a ", 100.0), ["a "]);
    }

    #[test]
    fn empty_text_has_no_lines() {
        assert!(geneva("", 100.0).is_empty());
    }

    #[test]
    fn a_width_with_no_room_still_terminates_one_character_a_line() {
        for width in [0.0, -5.0, f32::NAN, f32::NEG_INFINITY] {
            assert_eq!(geneva("ab c", width), ["a", "b", "c"], "{width}");
        }
        assert_eq!(geneva("ab c", f32::INFINITY), ["ab c"]);
    }

    #[test]
    fn charcoal_wraps_sooner_than_geneva() {
        let text = "aaaa aaaa";
        assert_eq!(wrap(&MonoMetrics, Font::Geneva, 10.0, text, 45.0), [text]);
        assert_eq!(
            wrap(&MonoMetrics, Font::Charcoal, 10.0, text, 45.0),
            ["aaaa", "aaaa"]
        );
    }

    #[test]
    fn borrowed_and_shared_metrics_are_metrics() {
        fn measure(metrics: impl TextMetrics) -> (f32, f32) {
            (
                metrics.width(Font::Geneva, 10.0, "ab"),
                metrics.line_height(Font::Geneva, 10.0),
            )
        }
        assert_eq!(measure(&MonoMetrics as &dyn TextMetrics), (10.0, 12.0));
        assert_eq!(measure(Rc::new(MonoMetrics)), (10.0, 12.0));
    }
}
