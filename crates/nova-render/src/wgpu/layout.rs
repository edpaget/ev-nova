//! Lays text out with whole-unit advances, as QuickDraw does with
//! fractional widths off: the one layout both the renderer and
//! [`GlyphonMetrics`](super::GlyphonMetrics) use, so text is drawn exactly
//! as it is measured.
//!
//! Text is shaped at its logical size, so every advance is in logical
//! units; the renderer draws the buffer at the UI scale. Each glyph
//! advances its [`whole_advance`]: its face's device width at the size
//! when the face's `hdmx` records one, else its linear advance rounded.
//!
//! cosmic-text has no per-glyph advance to set, so the layout takes two
//! passes over one [`Buffer`]:
//!
//! 1. Shape with no wrap and no metrics hinting, and read each glyph's
//!    linear advance. Hinting would round the advances it lays out but
//!    not the ones wrapping and line widths sum, so pass 1 never reads a
//!    rounded one.
//! 2. Give each cluster a letter-spacing span of the difference between
//!    its whole advance and its linear one, and shape again with the wrap
//!    width and hinting on. Wrapping, line widths and glyph positions all
//!    see the whole advances, and hinting rounds away the float residue
//!    the spacing leaves in the positions.

use std::ops::Range;

use glyphon::cosmic_text::{AttrsList, Hinting, LayoutGlyph};
use glyphon::{Buffer, FontSystem, Metrics, Shaping};
use nova_view::Font;
use nova_view::text::LINE_HEIGHT;

use super::fonts::{Families, attrs};
use crate::fonts::{device_width, whole_advance};

/// `text` in `font` at `size` logical units, laid out with whole advances
/// and wrapped at `wrap` logical units when it has one; `None` when no
/// face draws `font`. The caller checks the size gives a drawable line
/// (a positive, finite line height), which the shaper needs.
pub(super) fn lay_out_text(
    font_system: &mut FontSystem,
    families: &Families,
    font: Font,
    size: f32,
    wrap: Option<f32>,
    text: &str,
) -> Option<Buffer> {
    let family = families.family(font)?;
    let attrs = attrs(family);
    let mut buffer = Buffer::new(font_system, Metrics::new(size, LINE_HEIGHT * size));
    buffer.set_size(None, None);
    buffer.set_hinting(Hinting::Disabled);
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    let spacings: Vec<(usize, Vec<Spacing>)> = buffer
        .layout_runs()
        .map(|run| {
            let glyphs = run.glyphs.iter().map(|glyph| Advance {
                cluster: glyph.start..glyph.end,
                linear: glyph.w,
                whole: whole_advance(glyph.w, device(families, glyph)),
            });
            (run.line_i, spacing(glyphs, size))
        })
        .collect();
    for (line, clusters) in spacings {
        let mut list = AttrsList::new(&attrs);
        for (cluster, spacing) in clusters {
            list.add_span(cluster, &attrs.clone().letter_spacing(spacing));
        }
        buffer.lines[line].set_attrs_list(list);
    }
    buffer.set_size(wrap.map(whole_wrap), None);
    buffer.set_hinting(Hinting::Enabled);
    buffer.shape_until_scroll(font_system, false);
    Some(buffer)
}

/// `glyph`'s device width at its size, from its own face's widths (a
/// fallback face's for a glyph its run's face lacks).
fn device(families: &Families, glyph: &LayoutGlyph) -> Option<u8> {
    let widths = families.device_widths(glyph.font_id)?;
    device_width(widths, glyph.font_size, glyph.glyph_id)
}

/// One laid-out glyph's advances, in logical units.
#[derive(Clone, Debug, PartialEq)]
struct Advance {
    /// The text its cluster covers.
    cluster: Range<usize>,
    /// The shaper's linear advance.
    linear: f32,
    /// The whole advance it should have.
    whole: f32,
}

/// A cluster's text and its letter spacing, in ems.
type Spacing = (Range<usize>, f32);

/// The letter spacing, in ems of `size`, that turns each cluster's linear
/// advance into its whole one: one entry per cluster, in order. A cluster
/// of several glyphs (a base and its marks) shares its spacing among
/// them, as the shaper adds a cluster's spacing to each of its glyphs.
fn spacing(glyphs: impl IntoIterator<Item = Advance>, size: f32) -> Vec<Spacing> {
    let mut clusters: Vec<(Range<usize>, f32, u16)> = Vec::new();
    for glyph in glyphs {
        let extra = glyph.whole - glyph.linear;
        match clusters.last_mut() {
            Some((cluster, sum, count)) if cluster.start == glyph.cluster.start => {
                *sum += extra;
                *count += 1;
            }
            _ => clusters.push((glyph.cluster, extra, 1)),
        }
    }
    clusters
        .into_iter()
        .map(|(cluster, extra, count)| (cluster, extra / (f32::from(count) * size)))
        .collect()
}

/// The wrap width to lay out at for a `wrap` logical units wide: halfway
/// past its whole part. Every line's width is a whole number of units
/// plus the residue the letter spacing leaves, far under half a unit, so
/// a line fits this exactly when its whole width fits `wrap`, as
/// [`nova_view::text::wrap`] measures it.
fn whole_wrap(wrap: f32) -> f32 {
    wrap.floor() + 0.5
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn advance(cluster: Range<usize>, linear: f32, whole: f32) -> Advance {
        Advance {
            cluster,
            linear,
            whole,
        }
    }

    #[test]
    fn each_cluster_is_spaced_by_its_whole_advance_less_its_linear_one_in_ems() {
        let spaced = spacing(
            [
                advance(0..1, 2.5, 3.0),
                advance(1..2, 6.25, 6.0),
                advance(2..3, 4.0, 4.0),
            ],
            10.0,
        );
        assert_eq!(spaced, [(0..1, 0.05), (1..2, -0.025), (2..3, 0.0)]);
    }

    #[test]
    fn a_clusters_glyphs_share_its_spacing() {
        // A base and a mark on it, then another base: the cluster's two
        // glyphs are each spaced half its difference.
        let spaced = spacing(
            [
                advance(0..3, 5.5, 6.0),
                advance(0..3, 0.5, 1.0),
                advance(3..4, 2.0, 2.0),
            ],
            10.0,
        );
        assert_eq!(spaced, [(0..3, 0.05), (3..4, 0.0)]);
    }

    #[test]
    fn no_glyphs_have_no_spacing() {
        assert_eq!(spacing([], 10.0), []);
    }

    #[test]
    fn a_wrap_width_fits_exactly_the_whole_widths_it_holds() {
        assert_eq!(whole_wrap(100.0), 100.5);
        assert_eq!(whole_wrap(99.9), 99.5);
        assert_eq!(whole_wrap(0.0), 0.5);
        assert_eq!(whole_wrap(f32::INFINITY), f32::INFINITY);
    }
}
