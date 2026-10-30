//! [`TextMetrics`] by glyphon: text measured exactly as the renderer
//! lays it out, with no GPU.
//!
//! Widths are whole logical units, as QuickDraw's are with fractional
//! widths off: each glyph advances its face's device width or its linear
//! advance rounded ([`lay_out_text`]), with no kerning or ligature. So a
//! width is proportional to the size only within a unit a character,
//! which the port's contract allows.

use std::cell::RefCell;
use std::fmt;

use glyphon::FontSystem;
use nova_view::Font;
use nova_view::text::{LINE_HEIGHT, TextMetrics};

use super::fonts::{Families, font_system};
use super::layout::lay_out_text;
use crate::fonts::FontFaces;

/// Measures text in the faces the renderer draws it in: the same font
/// system (built from the same [`FontFaces`]), the same choice of face
/// ([`face_for`](crate::face_for)) and the same layout, with no wrapping.
/// A font with no face to draw it in measures 0 wide, as it draws nothing.
pub struct GlyphonMetrics {
    /// Shaping needs the font system mutably; measuring does not change
    /// what it measures.
    font_system: RefCell<FontSystem>,
    families: Families,
}

impl GlyphonMetrics {
    /// Metrics over `faces` alone: no system font is loaded.
    #[must_use]
    pub fn new(faces: &FontFaces) -> Self {
        let (font_system, families) = font_system(faces);
        Self {
            font_system: RefCell::new(font_system),
            families,
        }
    }
}

impl fmt::Debug for GlyphonMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlyphonMetrics")
            .field("charcoal_loaded", &self.families.charcoal_loaded())
            .finish_non_exhaustive()
    }
}

impl TextMetrics for GlyphonMetrics {
    /// The widest laid-out line of `text`, laid out as the renderer lays
    /// out a run with no wrap width: a whole number of units. Empty text,
    /// a size that is not a positive, finite number (which the shaper
    /// cannot take), and a font with no face are 0 wide.
    fn width(&self, font: Font, size: f32, text: &str) -> f32 {
        let line_height = self.line_height(font, size);
        if text.is_empty() || !(line_height.is_finite() && line_height > 0.0) {
            return 0.0;
        }
        let mut font_system = self.font_system.borrow_mut();
        let Some(buffer) = lay_out_text(&mut font_system, &self.families, font, size, None, text)
        else {
            return 0.0;
        };
        // Each advance is whole; rounding drops the float residue the
        // layout's letter spacing leaves in the sum.
        buffer
            .layout_runs()
            .map(|run| run.line_w.round())
            .fold(0.0, f32::max)
    }

    fn line_height(&self, _font: Font, size: f32) -> f32 {
        LINE_HEIGHT * size
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_data::fonts::fixture::{block_font, block_sfnt, hdmx};
    use nova_view::text::fixture::check_contract;

    use super::*;
    use crate::fonts::FALLBACK_FONT;

    /// The bundled fallback for Geneva, and the block font for Charcoal:
    /// every character a 1-em advance.
    fn both() -> GlyphonMetrics {
        GlyphonMetrics::new(&FontFaces::bundled().with_charcoal(block_font("Charcoal")))
    }

    #[test]
    fn it_meets_the_port_contract() {
        assert_eq!(check_contract(&both()), Ok(()));
    }

    #[test]
    fn empty_text_is_zero_wide() {
        let metrics = both();
        for font in [Font::Geneva, Font::Charcoal, Font::Times] {
            assert_eq!(metrics.width(font, 10.0, ""), 0.0, "{font:?}");
        }
    }

    #[test]
    fn width_grows_strictly_with_the_text_in_both_fonts() {
        let metrics = both();
        for font in [Font::Geneva, Font::Charcoal, Font::Times] {
            let widths: Vec<f32> = (1..=6)
                .map(|n| metrics.width(font, 10.0, &"Kestrel".repeat(n)))
                .collect();
            assert!(widths[0] > 0.0, "{font:?}: {widths:?}");
            assert!(
                widths.windows(2).all(|pair| pair[0] < pair[1]),
                "{font:?}: {widths:?}"
            );
        }
    }

    #[test]
    fn the_line_height_is_the_shared_rule_and_stable() {
        let metrics = both();
        for font in [Font::Geneva, Font::Charcoal, Font::Times] {
            for size in [9.0, 10.0, 12.0, 18.0] {
                let line = metrics.line_height(font, size);
                assert_eq!(line, LINE_HEIGHT * size, "{font:?} at {size}");
                assert_eq!(metrics.line_height(font, size), line, "stable");
            }
        }
    }

    #[test]
    fn doubling_the_size_doubles_width_and_line_height() {
        let metrics = both();
        let text = "The quick brown fox";
        // Each glyph rounds to a whole unit on its own, so within 1% plus
        // a unit a character.
        let slack = text.chars().count() as f32;
        for font in [Font::Geneva, Font::Charcoal, Font::Times] {
            let (one, two) = (
                metrics.width(font, 10.0, text),
                metrics.width(font, 20.0, text),
            );
            assert!(
                (two - 2.0 * one).abs() <= 0.01 * 2.0 * one + slack,
                "{one} {two}"
            );
            assert_eq!(
                metrics.line_height(font, 20.0),
                2.0 * metrics.line_height(font, 10.0)
            );
        }
    }

    #[test]
    fn charcoal_measures_in_its_own_face() {
        let metrics = both();
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "MMM"), 30.0);
        assert_eq!(metrics.width(Font::Charcoal, 12.0, "iii"), 36.0);
        let geneva = metrics.width(Font::Geneva, 10.0, "MMM");
        assert!(geneva > 0.0 && geneva != 30.0, "{geneva}");
    }

    #[test]
    fn with_no_charcoal_face_charcoal_measures_as_geneva() {
        let metrics = GlyphonMetrics::new(&FontFaces::bundled());
        for text in ["MMM", "Kestrel", "iii"] {
            assert_eq!(
                metrics.width(Font::Charcoal, 10.0, text),
                metrics.width(Font::Geneva, 10.0, text),
                "{text}"
            );
        }
        assert!(metrics.width(Font::Geneva, 10.0, "MMM") > 0.0);
    }

    #[test]
    fn with_no_face_at_all_text_is_zero_wide() {
        let metrics = GlyphonMetrics::new(&FontFaces::new(&b"not a font"[..]));
        for font in [Font::Geneva, Font::Charcoal, Font::Times] {
            assert_eq!(metrics.width(font, 10.0, "Kestrel"), 0.0, "{font:?}");
            assert_eq!(metrics.line_height(font, 10.0), 12.0);
        }
    }

    #[test]
    fn a_size_with_no_drawable_lines_is_zero_wide() {
        let metrics = both();
        for size in [0.0, -10.0, f32::NAN, f32::INFINITY, f32::MAX] {
            assert_eq!(metrics.width(Font::Geneva, size, "Kestrel"), 0.0, "{size}");
        }
    }

    #[test]
    fn times_measures_in_the_bundled_serif() {
        let metrics = GlyphonMetrics::new(&FontFaces::bundled());
        let times = metrics.width(Font::Times, 18.0, "Kestrel MMM iii");
        let geneva = metrics.width(Font::Geneva, 18.0, "Kestrel MMM iii");
        assert!(times > 0.0, "{times}");
        assert_ne!(times, geneva);
        let without = GlyphonMetrics::new(&FontFaces::new(FALLBACK_FONT));
        assert_eq!(
            without.width(Font::Times, 18.0, "Kestrel MMM iii"),
            geneva,
            "with no serif, Times measures as Geneva"
        );
    }

    #[test]
    fn every_width_is_a_whole_number_of_units() {
        let metrics = both();
        for font in [Font::Geneva, Font::Times] {
            for size in [9.0, 10.0, 12.0] {
                for text in ["Kestrel", "The quick brown fox", "lll iii", "Wavy AV To."] {
                    let width = metrics.width(font, size, text);
                    assert!(width > 0.0, "{font:?} {size} {text:?}");
                    assert_eq!(width, width.round(), "{font:?} {size} {text:?}");
                }
            }
        }
    }

    #[test]
    fn each_character_advances_its_own_width_with_no_kerning_or_ligature() {
        let metrics = both();
        for font in [Font::Geneva, Font::Times] {
            for size in [9.0, 10.0, 12.0] {
                let width = |text| metrics.width(font, size, text);
                for (pair, (a, b)) in [("AV", ("A", "V")), ("To", ("T", "o")), ("fi", ("f", "i"))] {
                    assert_eq!(width(pair), width(a) + width(b), "{font:?} {size} {pair}");
                }
            }
        }
    }

    #[test]
    fn a_run_of_one_glyph_is_that_many_whole_advances() {
        // At 10pt the fallback's `l` is 2.58 units: three are 9 whole
        // units, not 7.74 rounded.
        let metrics = both();
        assert_eq!(metrics.width(Font::Geneva, 10.0, "l"), 3.0);
        assert_eq!(metrics.width(Font::Geneva, 10.0, "lll"), 9.0);
        for size in [9.0, 10.0, 12.0] {
            let one = metrics.width(Font::Geneva, size, "l");
            assert_eq!(
                metrics.width(Font::Geneva, size, "llll"),
                4.0 * one,
                "{size}"
            );
        }
    }

    /// The block font as Charcoal, its glyph 13 units wide at 10 ppem by
    /// its `hdmx`, against a linear advance of 10.
    fn widened() -> GlyphonMetrics {
        let charcoal = block_sfnt("Charcoal")
            .table(b"hdmx", hdmx(2, &[(10, &[0, 13])]))
            .build();
        GlyphonMetrics::new(&FontFaces::bundled().with_charcoal(charcoal))
    }

    #[test]
    fn a_face_with_device_widths_advances_by_them_at_their_ppem() {
        let metrics = widened();
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "M"), 13.0);
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "MMM"), 39.0);
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "MM\nMMMM\nM"), 52.0);
        // No record at 12 or at 10.5: the linear advance, rounded.
        assert_eq!(metrics.width(Font::Charcoal, 12.0, "iii"), 36.0);
        assert_eq!(metrics.width(Font::Charcoal, 10.5, "iii"), 33.0);
    }

    #[test]
    fn a_glyph_the_face_lacks_advances_by_its_fallbacks_width() {
        // `é` is not in the block font: the fallback draws it, with no
        // device widths, so it rounds.
        let metrics = widened();
        let fallback = metrics.width(Font::Geneva, 10.0, "é");
        assert!(fallback > 0.0);
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "MéM"), 26.0 + fallback);
    }

    #[test]
    fn several_lines_measure_the_widest() {
        let metrics = both();
        assert_eq!(metrics.width(Font::Charcoal, 10.0, "MM\nMMMM\nM"), 40.0);
    }

    #[test]
    fn debug_says_whether_charcoal_loaded() {
        assert_eq!(
            format!("{:?}", both()),
            "GlyphonMetrics { charcoal_loaded: true, .. }"
        );
        assert_eq!(
            format!("{:?}", GlyphonMetrics::new(&FontFaces::bundled())),
            "GlyphonMetrics { charcoal_loaded: false, .. }"
        );
    }
}
