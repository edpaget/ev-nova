//! Draws laid-out text through glyphon's custom glyphs, rasterised with
//! full hints by [`glyph_raster`](crate::glyph_raster).
//!
//! glyphon rasterises its text glyphs only through swash, whose hinting
//! mode ignores TrueType instructions in X, and offers no hook into it.
//! Its one hook is custom glyphs: caller-rasterised coverage masks drawn
//! through the same mask atlas, colour, shader and clip as text. So the
//! renderer still lays each run out with
//! [`lay_out_text`](super::layout::lay_out_text), then turns the buffer's
//! glyphs into custom glyphs ([`HintedGlyphs::custom_glyphs`]) and hands
//! glyphon an empty buffer beside them.
//!
//! Each glyph lands exactly where glyphon puts a text glyph
//! ([`place`]): glyphon's own formula, in physical pixels, with the
//! glyph's start rounded to a whole pixel as QuickDraw draws every glyph
//! from a whole-pixel pen. The custom glyphs' text area has its origin at
//! (0, 0) and a scale of 1, and every glyph's rectangle is whole physical
//! pixels, so glyphon moves none of them.
//!
//! A raster is keyed by its face, glyph and physical size ([`RasterKey`]).
//! [`GlyphIds`] hands each key the custom-glyph ID glyphon caches it by,
//! unique for the renderer's life, and [`HintedGlyphs`] keeps every raster
//! it has made, since glyphon asks for them again when its atlas grows.
//! Once every ID is taken, a glyph with a new key is not drawn: never
//! another glyph's raster in its place.

use std::collections::HashMap;
use std::sync::Arc;

use glyphon::fontdb::ID;
use glyphon::{
    Buffer, ContentType, CustomGlyph, CustomGlyphId, RasterizeCustomGlyphRequest,
    RasterizedCustomGlyph,
};

use super::fonts::Families;
use crate::glyph_raster::{GlyphRaster, HintedFace};

/// What a raster is of: a face (by fontdb ID), a glyph in it, and the
/// physical pixels per em (as bits).
pub(super) type RasterKey = (ID, u16, u32);

/// A glyph's rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Placed {
    pub(super) left: i32,
    pub(super) top: i32,
}

/// Where glyphon draws a text glyph, in physical pixels: the glyph's
/// physical position (`x`, `y`, already offset by the text area's origin
/// and scaled) and its subpixel bin, its line's baseline `line_y` in
/// logical units at `scale`, and its raster's placement.
///
/// The bin is rounded to a whole pixel, as the `PIXEL_FONT` flag rounded
/// it when swash rasterised glyphon's text; `line_y * scale` rounds as
/// glyphon rounds it.
pub(super) fn place(
    (x, y, x_bin): (i32, i32, f32),
    line_y: f32,
    scale: f32,
    raster: &GlyphRaster,
) -> Placed {
    Placed {
        left: x + x_bin.round() as i32 + raster.left,
        top: (line_y * scale).round() as i32 + y - raster.top,
    }
}

/// Custom-glyph IDs for raster keys: one per key, never reused.
pub(super) struct GlyphIds {
    ids: HashMap<RasterKey, CustomGlyphId>,
    capacity: usize,
}

impl GlyphIds {
    /// Every ID glyphon has room for.
    pub(super) fn new() -> Self {
        Self::with_capacity(usize::from(CustomGlyphId::MAX) + 1)
    }

    /// At most `capacity` IDs (and never more than glyphon has room for).
    fn with_capacity(capacity: usize) -> Self {
        Self {
            ids: HashMap::new(),
            capacity: capacity.min(usize::from(CustomGlyphId::MAX) + 1),
        }
    }

    /// The ID of `key`, handed out the first time it is asked for; `None`
    /// once every ID is taken by another key.
    pub(super) fn id(&mut self, key: RasterKey) -> Option<CustomGlyphId> {
        if let Some(&id) = self.ids.get(&key) {
            return Some(id);
        }
        if self.ids.len() >= self.capacity {
            return None;
        }
        let id = CustomGlyphId::try_from(self.ids.len()).ok()?;
        self.ids.insert(key, id);
        Some(id)
    }
}

/// A [`HintedFace`] for each face `families` loaded, by its fontdb ID.
fn hinted_faces(families: &Families) -> Vec<(ID, HintedFace)> {
    let sources = families.sources().iter();
    sources
        .filter_map(|(id, bytes, index)| Some((*id, HintedFace::new(Arc::clone(bytes), *index)?)))
        .collect()
}

/// The renderer's faces and every glyph raster made from them, by ID.
pub(super) struct HintedGlyphs {
    faces: Vec<(ID, HintedFace)>,
    ids: GlyphIds,
    /// The raster for each ID handed out, by ID.
    rasters: Vec<Option<GlyphRaster>>,
}

impl HintedGlyphs {
    /// Glyphs from every face `families` loaded.
    pub(super) fn of(families: &Families) -> Self {
        Self::new(hinted_faces(families))
    }

    /// Glyphs from `faces`, each by its fontdb ID.
    fn new(faces: Vec<(ID, HintedFace)>) -> Self {
        Self::with_ids(faces, GlyphIds::new())
    }

    fn with_ids(faces: Vec<(ID, HintedFace)>, ids: GlyphIds) -> Self {
        Self {
            faces,
            ids,
            rasters: Vec::new(),
        }
    }

    /// The ID and raster of `key`, rasterised the first time; `None` for a
    /// face this holds no file for, a glyph with no outline, or once every
    /// ID is taken.
    fn raster(&mut self, (face, glyph, ppem): RasterKey) -> Option<(CustomGlyphId, &GlyphRaster)> {
        let id = self.ids.id((face, glyph, ppem))?;
        let at = usize::from(id);
        if self.rasters.len() <= at {
            self.rasters.resize(at + 1, None);
            let hinted = self.faces.iter_mut().find(|(loaded, _)| *loaded == face);
            self.rasters[at] =
                hinted.and_then(|(_, hinted)| hinted.rasterise(glyph, f32::from_bits(ppem)));
        }
        Some((id, self.rasters[at].as_ref()?))
    }

    /// The custom glyphs that draw `buffer` with its origin at `origin`
    /// physical pixels and `scale` pixels per unit: one per glyph with ink,
    /// on the lines glyphon would draw inside `rows` (the clip's top and
    /// bottom, in physical pixels).
    pub(super) fn custom_glyphs(
        &mut self,
        buffer: &Buffer,
        origin: (f32, f32),
        scale: f32,
        rows: (i32, i32),
    ) -> Vec<CustomGlyph> {
        // glyphon's own test for a line it draws.
        let visible = |run: &glyphon::LayoutRun<'_>| {
            let start = (origin.1 + run.line_top * scale) as i32;
            let end = start + (run.line_height * scale) as i32;
            start <= rows.1 && rows.0 <= end
        };
        let mut glyphs = Vec::new();
        let runs = buffer.layout_runs().skip_while(|run| !visible(run));
        for run in runs.take_while(visible) {
            for glyph in run.glyphs {
                let physical = glyph.physical(origin, scale);
                let key = physical.cache_key;
                let raster_key = (key.font_id, key.glyph_id, key.font_size_bits);
                let Some((id, raster)) = self.raster(raster_key) else {
                    continue;
                };
                if raster.is_empty() {
                    continue;
                }
                let at = (physical.x, physical.y, key.x_bin.as_float());
                let placed = place(at, run.line_y, scale, raster);
                glyphs.push(CustomGlyph {
                    id,
                    left: placed.left as f32,
                    top: placed.top as f32,
                    width: raster.width as f32,
                    height: raster.height as f32,
                    color: glyph.color_opt,
                    snap_to_physical_pixel: true,
                    metadata: glyph.metadata,
                });
            }
        }
        glyphs
    }

    /// The coverage glyphon asks for: the raster made for `request`'s ID,
    /// when its size matches; `None` for any other request, so glyphon's
    /// size check never fails.
    pub(super) fn rasterize(
        &self,
        request: RasterizeCustomGlyphRequest,
    ) -> Option<RasterizedCustomGlyph> {
        let raster = self.rasters.get(usize::from(request.id))?.as_ref()?;
        let fits =
            u32::from(request.width) == raster.width && u32::from(request.height) == raster.height;
        fits.then(|| RasterizedCustomGlyph {
            data: raster.coverage.clone(),
            content_type: ContentType::Mask,
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use glyphon::{Attrs, Color, FontSystem, Metrics, Shaping, SubpixelBin};
    use nova_view::Font;

    use super::*;
    use crate::fonts::FontFaces;
    use crate::wgpu::fonts::font_system;
    use crate::wgpu::layout::lay_out_text;

    fn raster(left: i32, top: i32, width: u32, height: u32) -> GlyphRaster {
        GlyphRaster {
            left,
            top,
            width,
            height,
            coverage: vec![255; (width * height) as usize],
            advance: None,
        }
    }

    #[test]
    fn a_glyph_on_a_whole_pixel_is_placed_by_its_bearing_and_baseline() {
        let r = raster(1, 9, 2, 9);
        assert_eq!(
            place((10, 4, 0.0), 13.0, 1.0, &r),
            Placed { left: 11, top: 8 }
        );
        assert_eq!(
            place((20, 8, 0.0), 13.0, 2.0, &r),
            Placed { left: 21, top: 25 }
        );
    }

    #[test]
    fn a_fractional_bin_rounds_to_the_nearer_whole_pixel() {
        let r = raster(0, 9, 2, 9);
        for (bin, left) in [(0.0, 10), (0.25, 10), (0.5, 11), (0.75, 11)] {
            assert_eq!(place((10, 0, bin), 0.0, 1.0, &r).left, left, "bin {bin}");
        }
    }

    #[test]
    fn the_baseline_rounds_at_a_fractional_scale() {
        // 9.6 units at 1.40625 is 13.5 pixels, rounded away from zero.
        let r = raster(0, 8, 2, 9);
        assert_eq!(place((0, 3, 0.0), 9.6, 1.40625, &r).top, 14 + 3 - 8);
        // 13 units at 1.40625 is 18.28 pixels, rounded down.
        assert_eq!(place((0, 3, 0.0), 13.0, 1.40625, &r).top, 18 + 3 - 8);
    }

    #[test]
    fn a_negative_left_bearing_starts_left_of_the_pen() {
        let r = raster(-2, 9, 6, 9);
        assert_eq!(place((10, 0, 0.75), 0.0, 1.0, &r).left, 9);
    }

    fn key(glyph: u16) -> RasterKey {
        (ID::dummy(), glyph, 12.0f32.to_bits())
    }

    #[test]
    fn a_key_keeps_its_id_and_distinct_keys_get_distinct_ids() {
        let mut ids = GlyphIds::new();
        let a = ids.id(key(1)).expect("an id");
        let b = ids.id(key(2)).expect("an id");
        let c = ids.id((ID::dummy(), 1, 24.0f32.to_bits())).expect("an id");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
        assert_eq!(ids.id(key(1)), Some(a));
        assert_eq!(ids.id(key(2)), Some(b));
    }

    #[test]
    fn once_every_id_is_taken_a_new_key_gets_none_and_old_keys_keep_theirs() {
        let mut ids = GlyphIds::with_capacity(2);
        let a = ids.id(key(1)).expect("an id");
        let b = ids.id(key(2)).expect("an id");
        assert_eq!(ids.id(key(3)), None);
        assert_eq!(ids.id(key(1)), Some(a));
        assert_eq!(ids.id(key(2)), Some(b));
        assert_eq!(ids.id(key(3)), None);
    }

    #[test]
    fn glyphon_has_room_for_every_id_handed_out() {
        let mut ids = GlyphIds::with_capacity(usize::MAX);
        assert_eq!(ids.capacity, 65_536);
        for glyph in 0..=u16::MAX {
            assert_eq!(ids.id(key(glyph)), Some(glyph));
        }
        assert_eq!(ids.id((ID::dummy(), 0, 9.0f32.to_bits())), None);
    }

    /// The bundled faces' font system and the glyphs that draw from it.
    fn bundled() -> (FontSystem, Families, HintedGlyphs) {
        let (system, families) = font_system(&FontFaces::bundled());
        let glyphs = HintedGlyphs::of(&families);
        (system, families, glyphs)
    }

    const ALL_ROWS: (i32, i32) = (i32::MIN, i32::MAX);

    #[test]
    fn a_buffer_gives_one_custom_glyph_per_glyph_with_ink() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "ab c").expect("laid");
        let custom = glyphs.custom_glyphs(&buffer, (4.0, 6.0), 2.0, ALL_ROWS);
        assert_eq!(custom.len(), 3, "{custom:?}");
        let mut lefts: Vec<f32> = custom.iter().map(|glyph| glyph.left).collect();
        lefts.dedup();
        assert_eq!(lefts.len(), 3);
        for glyph in &custom {
            assert_eq!(glyph.color, None);
            assert!(glyph.snap_to_physical_pixel);
            assert!(glyph.width > 0.0 && glyph.height > 0.0);
            assert_eq!(glyph.left.fract(), 0.0);
            assert_eq!(glyph.top.fract(), 0.0);
        }
    }

    #[test]
    fn each_custom_glyph_is_placed_as_glyphon_places_its_text() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer = lay_out_text(
            &mut system,
            &families,
            Font::Geneva,
            9.0,
            Some(40.0),
            "Kestrel sass a",
        )
        .expect("laid");
        let (origin, scale) = ((3.5, 7.25), 1.40625);
        let custom = glyphs.custom_glyphs(&buffer, origin, scale, ALL_ROWS);
        let mut want = Vec::new();
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical(origin, scale);
                let key = physical.cache_key;
                let (_, raster) = glyphs
                    .raster((key.font_id, key.glyph_id, key.font_size_bits))
                    .expect("a raster");
                if raster.is_empty() {
                    continue;
                }
                let at = (physical.x, physical.y, key.x_bin.as_float());
                want.push(place(at, run.line_y, scale, raster));
            }
        }
        assert!(buffer.layout_runs().count() > 1, "wraps");
        let got: Vec<Placed> = custom
            .iter()
            .map(|glyph| Placed {
                left: glyph.left as i32,
                top: glyph.top as i32,
            })
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn a_glyph_is_rasterised_at_its_physical_size() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "l").expect("laid");
        let one = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, ALL_ROWS);
        let two = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 2.0, ALL_ROWS);
        assert_ne!(one[0].id, two[0].id);
        assert!(
            two[0].height >= 2.0 * one[0].height - 1.0,
            "{one:?} {two:?}"
        );
    }

    #[test]
    fn a_line_outside_the_clip_rows_gives_no_glyphs() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "a").expect("laid");
        assert_eq!(
            glyphs.custom_glyphs(&buffer, (0.0, 100.0), 1.0, (0, 50)),
            []
        );
        assert_eq!(glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, (50, 90)), []);
        assert_eq!(
            glyphs
                .custom_glyphs(&buffer, (0.0, 40.0), 1.0, (0, 50))
                .len(),
            1
        );
    }

    #[test]
    fn a_glyph_set_its_own_colour_keeps_it() {
        let (mut system, families, mut glyphs) = bundled();
        let red = Color::rgb(255, 0, 0);
        let family = families.family(Font::Geneva).expect("a family");
        let mut buffer = Buffer::new(&mut system, Metrics::new(12.0, 14.0));
        let attrs = Attrs::new().family(family).color(red);
        buffer.set_text("a", &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut system, false);
        let custom = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, ALL_ROWS);
        assert_eq!(custom[0].color, Some(red));
    }

    #[test]
    fn a_face_with_no_file_draws_no_glyphs() {
        let (mut system, families, _) = bundled();
        let mut glyphs = HintedGlyphs::new(Vec::new());
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "ab").expect("laid");
        assert_eq!(glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, ALL_ROWS), []);
    }

    #[test]
    fn with_every_id_taken_a_new_glyph_is_left_out_not_drawn_as_another() {
        let (mut system, families, _) = bundled();
        let mut glyphs =
            HintedGlyphs::with_ids(hinted_faces(&families), GlyphIds::with_capacity(1));
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "ab").expect("laid");
        let custom = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, ALL_ROWS);
        assert_eq!(custom.len(), 1, "{custom:?}");
        let a = lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "a").expect("laid");
        assert_eq!(glyphs.custom_glyphs(&a, (0.0, 0.0), 1.0, ALL_ROWS), custom);
    }

    fn request(id: CustomGlyphId, width: u16, height: u16) -> RasterizeCustomGlyphRequest {
        RasterizeCustomGlyphRequest {
            id,
            width,
            height,
            x_bin: SubpixelBin::Zero,
            y_bin: SubpixelBin::Zero,
            scale: 1.0,
        }
    }

    #[test]
    fn glyphon_is_handed_the_raster_made_for_an_id() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "lW").expect("laid");
        let custom = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 2.0, ALL_ROWS);
        for glyph in custom {
            let (w, h) = (glyph.width as u16, glyph.height as u16);
            let raster = glyphs.rasterize(request(glyph.id, w, h)).expect("made");
            assert_eq!(raster.content_type, ContentType::Mask);
            assert_eq!(raster.data.len(), usize::from(w) * usize::from(h));
            assert!(raster.data.contains(&255));
        }
    }

    #[test]
    fn glyphon_is_handed_nothing_for_an_unknown_id_or_another_size() {
        let (mut system, families, mut glyphs) = bundled();
        let buffer =
            lay_out_text(&mut system, &families, Font::Geneva, 12.0, None, "l").expect("laid");
        let glyph = glyphs.custom_glyphs(&buffer, (0.0, 0.0), 1.0, ALL_ROWS)[0];
        let (w, h) = (glyph.width as u16, glyph.height as u16);
        assert!(glyphs.rasterize(request(glyph.id + 1, w, h)).is_none());
        assert!(glyphs.rasterize(request(glyph.id, w + 1, h)).is_none());
        assert!(glyphs.rasterize(request(glyph.id, w, h + 1)).is_none());
        assert!(glyphs.rasterize(request(glyph.id, w, h)).is_some());
    }

    #[test]
    fn a_glyph_with_no_outline_has_an_id_but_no_raster() {
        let (_, families, _) = bundled();
        let mut glyphs = HintedGlyphs::new(hinted_faces(&families));
        let face = families.sources()[0].0;
        assert!(glyphs.raster((face, u16::MAX, 12.0f32.to_bits())).is_none());
        assert!(glyphs.rasterize(request(0, 1, 1)).is_none());
    }
}
