//! Text laid out with whole advances, drawn through the wgpu adapter:
//! each glyph advances a whole logical unit times the scale, and text is
//! drawn exactly where `GlyphonMetrics` measures it, wrapped or not. Needs
//! a GPU adapter; skips, passing, without one.

use crate::wgpu_offscreen::{Picture, STEMS, TEXT_AT, draw_list, draw_text};

use nova_data::fonts::fixture::{block_sfnt, hdmx};
use nova_render::fonts::FontFaces;
use nova_render::wgpu::GlyphonMetrics;
use nova_view::text::{TextMetrics, wrap};
use nova_view::{Color, DrawList, Font, Point};

/// The bundled faces, with the block font as Charcoal: its glyph 13 units
/// wide at 10 ppem by its `hdmx`, against a linear advance of 10.
fn widened() -> FontFaces {
    let charcoal = block_sfnt("Charcoal")
        .table(b"hdmx", hdmx(2, &[(10, &[0, 13])]))
        .build();
    FontFaces::bundled().with_charcoal(charcoal)
}

/// Where the tests' text starts.
fn at() -> Point {
    Point::new(TEXT_AT.0, TEXT_AT.1)
}

/// Each of `columns` counted from the first.
fn from_first(columns: &[u32]) -> Vec<u32> {
    columns.iter().map(|&x| x - columns[0]).collect()
}

#[test]
fn at_2x_each_stem_sits_at_twice_its_1x_column() {
    let faces = FontFaces::bundled();
    let (Some(one), Some(two)) = (
        draw_text(&faces, Font::Geneva, STEMS, at(), 12.0, 1),
        draw_text(&faces, Font::Geneva, STEMS, at(), 12.0, 2),
    ) else {
        return;
    };
    let (one, two) = (one.stem_columns(), two.stem_columns());
    assert_eq!(one.len(), STEMS.len(), "{one:?}");
    let doubled: Vec<u32> = from_first(&one).iter().map(|x| 2 * x).collect();
    assert_eq!(from_first(&two), doubled, "1x {one:?}, 2x {two:?}");
}

/// `text` then an `l` drawn as one run, and `text` with an `l` drawn after
/// it where `GlyphonMetrics` says `text` ends, in `font` at `size` and
/// `scale`; `None` with no GPU adapter.
pub(crate) fn whole_and_split(
    faces: &FontFaces,
    font: Font,
    size: f32,
    text: &str,
    scale: u32,
) -> Option<(Picture, Picture)> {
    let width = GlyphonMetrics::new(faces).width(font, size, text);
    let mut whole = DrawList::new();
    whole.text_in(font, format!("{text}l"), at(), size, None, Color::WHITE);
    let mut split = DrawList::new();
    split.text_in(font, text, at(), size, None, Color::WHITE);
    let after = Point::new(at().x + width, at().y);
    split.text_in(font, "l", after, size, None, Color::WHITE);
    Some((
        draw_list(faces, &whole, scale)?,
        draw_list(faces, &split, scale)?,
    ))
}

#[test]
fn text_is_drawn_exactly_as_wide_as_it_measures() {
    let faces = widened();
    for (font, size, text) in [
        (Font::Geneva, 9.0, "Kestrel"),
        (Font::Geneva, 10.0, "lll"),
        (Font::Geneva, 12.0, "Wavy To."),
        (Font::Times, 12.0, "Kestrel"),
        (Font::Charcoal, 10.0, "MMM"),
        (Font::Charcoal, 12.0, "MMM"),
    ] {
        for scale in [1, 2] {
            let Some((whole, split)) = whole_and_split(&faces, font, size, text, scale) else {
                return;
            };
            assert!(!whole.lit().is_empty(), "{font:?} {size} {text:?} {scale}x");
            assert_eq!(whole, split, "{font:?} {size} {text:?} at {scale}x");
        }
    }
}

/// `text` wrapped by the renderer at `width`, and the lines
/// [`nova_view::text::wrap`] breaks it into with `GlyphonMetrics`, each
/// drawn as a run of its own a line below the last; `None` with no GPU
/// adapter.
fn wrapped_and_lines(
    faces: &FontFaces,
    font: Font,
    size: f32,
    (text, width): (&str, f32),
    scale: u32,
) -> Option<(Picture, Picture, usize)> {
    let metrics = GlyphonMetrics::new(faces);
    let lines = wrap(&metrics, font, size, text, width);
    let mut wrapped = DrawList::new();
    wrapped.text_in(font, text, at(), size, Some(width), Color::WHITE);
    let mut each = DrawList::new();
    for (row, line) in lines.iter().enumerate() {
        let y = at().y + row as f32 * metrics.line_height(font, size);
        each.text_in(font, line, Point::new(at().x, y), size, None, Color::WHITE);
    }
    Some((
        draw_list(faces, &wrapped, scale)?,
        draw_list(faces, &each, scale)?,
        lines.len(),
    ))
}

#[test]
fn renderer_wrapped_text_breaks_where_the_measured_wrap_does() {
    let faces = widened();
    // Each fits on one line by its linear widths (the block font's 50 of
    // 60; three `l`s of the fallback are 7.74 at 10pt, so 18.08 of 20)
    // and not by its whole ones (65; 9 + 3 + 9 = 21).
    for (font, text, width) in [
        (Font::Charcoal, "MM MM", 60.0),
        (Font::Geneva, "lll lll", 20.0),
    ] {
        for scale in [1, 2] {
            let Some((wrapped, lines, count)) =
                wrapped_and_lines(&faces, font, 10.0, (text, width), scale)
            else {
                return;
            };
            assert_eq!(count, 2, "{font:?} {text:?}");
            assert_eq!(wrapped, lines, "{font:?} {text:?} at {scale}x");
        }
    }
}
