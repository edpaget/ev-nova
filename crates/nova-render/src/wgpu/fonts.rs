//! The glyphon font system both text users build: the renderer, which
//! draws text, and [`GlyphonMetrics`](super::GlyphonMetrics), which
//! measures it the same way.

use std::sync::Arc;

use glyphon::cosmic_text::CacheKeyFlags;
use glyphon::fontdb::{Database, Source};
use glyphon::{Attrs, Family, FontSystem};
use nova_view::Font;

use crate::fonts::{Face, FontFaces, Loaded, face_for};

/// The family name of each loaded face, as fontdb read it.
pub(super) struct Families {
    charcoal: Option<String>,
    fallback: Option<String>,
    serif: Option<String>,
}

impl Families {
    /// The family that draws `font`, chosen by [`face_for`] among the faces
    /// that loaded; `None` when none did.
    pub(super) fn family(&self, font: Font) -> Option<Family<'_>> {
        let loaded = Loaded {
            charcoal: self.charcoal.is_some(),
            fallback: self.fallback.is_some(),
            serif: self.serif.is_some(),
        };
        let name = match face_for(font, loaded)? {
            Face::Charcoal => &self.charcoal,
            Face::Fallback => &self.fallback,
            Face::Serif => &self.serif,
        };
        name.as_deref().map(Family::Name)
    }

    /// Whether Charcoal's font file loaded a face.
    pub(super) fn charcoal_loaded(&self) -> bool {
        self.charcoal.is_some()
    }
}

/// The attributes both text users shape `family` with, so text is measured
/// exactly as it is drawn.
///
/// Each glyph is rasterised starting on a whole physical pixel, as
/// QuickDraw draws every glyph from a whole-pixel pen. Left to the
/// shaper's fractional advances, consecutive glyphs of one run fall in
/// different quarter-pixel bins and the same stem smears over a different
/// split of columns each time. The flag is read only when a glyph is
/// rasterised: advances, line widths and wrapping are unchanged, and a
/// glyph moves by at most half a pixel, never cumulatively.
pub(super) fn attrs(family: Family<'_>) -> Attrs<'_> {
    Attrs::new()
        .family(family)
        .cache_key_flags(CacheKeyFlags::PIXEL_FONT)
}

/// A font system holding only `faces`, with the fallback as its
/// sans-serif family, and each face's family name: the fallback's,
/// Charcoal's and the serif's.
pub(super) fn font_system(faces: &FontFaces) -> (FontSystem, Families) {
    let mut db = Database::new();
    let mut load = |bytes: &Arc<[u8]>| {
        let ids = db.load_font_source(Source::Binary(Arc::new(Arc::clone(bytes))));
        ids.first()
            .and_then(|&id| db.face(id))
            .and_then(|face| face.families.first())
            .map(|(name, _)| name.clone())
    };
    let fallback = load(faces.fallback());
    let charcoal = faces.charcoal().and_then(&mut load);
    let serif = faces.serif().and_then(&mut load);
    if let Some(name) = &fallback {
        db.set_sans_serif_family(name.clone());
    }
    let system = FontSystem::new_with_locale_and_db("en-US".to_owned(), db);
    (
        system,
        Families {
            charcoal,
            fallback,
            serif,
        },
    )
}
