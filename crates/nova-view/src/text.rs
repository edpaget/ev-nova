//! Text measurement: the [`TextMetrics`] port, the shared line-height rule
//! and word wrap.
//!
//! Everything is in logical units. The port has no scale: the renderer
//! shapes each text run at its logical size and draws it at the UI scale,
//! multiplying its origin and glyph positions by the scale, so text
//! scales with the rest of the UI. Widths may be whole units, as the
//! glyphon adapter's (`nova_render::wgpu::GlyphonMetrics`) are, like
//! QuickDraw's with fractional widths off: each character rounds on its
//! own, so a width is proportional to the size only within one unit per
//! character.
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
    /// at `size`, in logical units. The empty string is 0 wide. An adapter
    /// may give whole units, as QuickDraw's widths are with fractional
    /// widths off.
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

/// The ellipsis `TruncString` ends a truncated text with.
pub const ELLIPSIS: &str = "…";

/// `text` cut to fit `width`, measured by `metrics` in `font` at `size`,
/// as `TruncString` with `truncEnd` cuts it: a text that fits comes back
/// whole, and one that doesn't keeps the longest leading run of whole
/// characters that fits with [`ELLIPSIS`] after it. With room for no
/// more than the ellipsis it is the ellipsis alone, and with less (or a
/// width that is not a number) nothing.
#[must_use]
pub fn truncate_end(
    metrics: &impl TextMetrics,
    font: Font,
    size: f32,
    text: &str,
    width: f32,
) -> String {
    let fits = |candidate: &str| metrics.width(font, size, candidate) <= width;
    if fits(text) {
        return text.to_owned();
    }
    let ends = std::iter::once(0).chain(text.char_indices().skip(1).map(|(at, _)| at));
    ends.map(|end| format!("{}{ELLIPSIS}", &text[..end]))
        .take_while(|candidate| fits(candidate))
        .last()
        .unwrap_or_default()
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
    // Whether a wrap point has been passed: after one, a fresh line drops
    // its leading spaces, while the paragraph's own leading spaces stay.
    let mut wrapped = false;
    for word in paragraph.split(' ') {
        if wrapped && line.is_none() && word.is_empty() {
            // A space at the wrap point.
            continue;
        }
        let candidate = match &line {
            None => word.to_owned(),
            Some(line) => format!("{line} {word}"),
        };
        if fits(&candidate) {
            line = Some(candidate);
            continue;
        }
        wrapped = true;
        if let Some(full) = line.take() {
            let full = full.trim_end_matches(' ');
            if !full.is_empty() {
                lines.push(full.to_owned());
            }
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
    fn a_run_of_spaces_at_a_wrap_point_leaves_no_blank_line() {
        assert_eq!(geneva("abc   def", 15.0), ["abc", "def"]);
        assert_eq!(geneva("     abc", 10.0), ["ab", "c"]);
    }

    #[test]
    fn a_run_of_spaces_at_a_wrap_point_leaves_no_leading_space() {
        assert_eq!(geneva("abc    def", 20.0), ["abc", "def"]);
        assert_eq!(
            geneva("Producer:    Matt Burch", 50.0),
            ["Producer:", "Matt Burch"]
        );
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

    /// `text` cut to `width` in Geneva 10: 5 units a character.
    fn cut(text: &str, width: f32) -> String {
        truncate_end(&MonoMetrics, Font::Geneva, 10.0, text, width)
    }

    #[test]
    fn a_text_that_fits_is_not_truncated() {
        assert_eq!(cut("abcd", 20.0), "abcd");
        assert_eq!(cut("", 0.0), "");
        assert_eq!(cut("abcd", f32::INFINITY), "abcd");
    }

    #[test]
    fn a_text_too_wide_keeps_its_longest_prefix_that_fits_with_the_ellipsis() {
        assert_eq!(cut("abcdef", 25.0), "abcd…");
        assert_eq!(cut("abcdef", 24.9), "abc…");
        assert_eq!(cut("abcde", 24.9), "abc…");
        assert_eq!(cut("äöüßx", 15.0), "äö…", "whole characters");
        assert_eq!(ELLIPSIS, "…");
    }

    #[test]
    fn with_room_for_the_ellipsis_alone_it_is_the_ellipsis_and_with_less_nothing() {
        assert_eq!(cut("abc", 5.0), "…");
        assert_eq!(cut("abc", 9.9), "…");
        for width in [4.9, 0.0, -1.0, f32::NAN] {
            assert_eq!(cut("abc", width), "", "{width}");
        }
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
