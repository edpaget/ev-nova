//! [`TextMetrics`] by glyphon: text measured exactly as the renderer
//! shapes it, with no GPU.

use std::cell::RefCell;
use std::fmt;

use glyphon::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use nova_view::Font;
use nova_view::text::{LINE_HEIGHT, TextMetrics};

use super::fonts::{Families, font_system};
use crate::fonts::FontFaces;

/// Measures text in the faces the renderer draws it in: the same font
/// system (built from the same [`FontFaces`]), the same choice of face
/// ([`face_for`](crate::face_for)), the same metrics and shaping, and no
/// wrapping. A font with no face to draw it in measures 0 wide, as it draws
/// nothing.
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
    /// The widest laid-out line of `text`, shaped as the renderer shapes a
    /// run with no wrap width. Empty text, a size that is not a positive,
    /// finite number (which the shaper cannot take), and a font with no
    /// face are 0 wide.
    fn width(&self, font: Font, size: f32, text: &str) -> f32 {
        let line_height = self.line_height(font, size);
        if text.is_empty() || !(line_height.is_finite() && line_height > 0.0) {
            return 0.0;
        }
        let Some(family) = self.families.family(font) else {
            return 0.0;
        };
        let mut font_system = self.font_system.borrow_mut();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(size, line_height));
        buffer.set_size(None, None);
        buffer.set_text(text, &Attrs::new().family(family), Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);
        buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0, f32::max)
    }

    fn line_height(&self, _font: Font, size: f32) -> f32 {
        LINE_HEIGHT * size
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_data::fonts::fixture::block_font;
    use nova_view::text::fixture::check_contract;

    use super::*;

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
        for font in [Font::Geneva, Font::Charcoal] {
            assert_eq!(metrics.width(font, 10.0, ""), 0.0, "{font:?}");
        }
    }

    #[test]
    fn width_grows_strictly_with_the_text_in_both_fonts() {
        let metrics = both();
        for font in [Font::Geneva, Font::Charcoal] {
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
        for font in [Font::Geneva, Font::Charcoal] {
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
        for font in [Font::Geneva, Font::Charcoal] {
            let (one, two) = (
                metrics.width(font, 10.0, "The quick brown fox"),
                metrics.width(font, 20.0, "The quick brown fox"),
            );
            assert!((two - 2.0 * one).abs() <= 0.01 * 2.0 * one, "{one} {two}");
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
        for font in [Font::Geneva, Font::Charcoal] {
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
