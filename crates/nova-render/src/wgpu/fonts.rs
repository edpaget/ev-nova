//! The glyphon font system both text users build: the renderer, which
//! draws text, and [`GlyphonMetrics`](super::GlyphonMetrics), which
//! measures it the same way. Both lay text out through
//! [`lay_out_text`](super::layout::lay_out_text), with the attributes
//! [`attrs`] gives and each face's [`DeviceWidths`], which [`Families`]
//! keeps by the face's fontdb ID, beside the face's font file, which the
//! renderer rasterises its glyphs from
//! ([`hinted_text`](super::hinted_text)).

use std::sync::Arc;

use glyphon::cosmic_text::{FeatureTag, FontFeatures};
use glyphon::fontdb::{Database, ID, Source};
use glyphon::{Attrs, Family, FontSystem};
use nova_data::fonts::DeviceWidths;
use nova_view::Font;

use crate::fonts::{Face, FontFaces, Loaded, face_for};

/// The family name of each loaded face, as fontdb read it, and the device
/// widths of every face loaded.
pub(super) struct Families {
    charcoal: Option<String>,
    fallback: Option<String>,
    serif: Option<String>,
    /// Each loaded face's fontdb ID and the widths its `hdmx` records
    /// (none for a face with no `hdmx`).
    widths: Vec<(ID, DeviceWidths)>,
    /// Each loaded face's fontdb ID, its font file and its index in it.
    sources: Vec<(ID, Arc<[u8]>, u32)>,
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

    /// The device widths of the loaded face `id`; `None` for a face this
    /// system did not load.
    pub(super) fn device_widths(&self, id: ID) -> Option<&DeviceWidths> {
        self.widths
            .iter()
            .find(|(face, _)| *face == id)
            .map(|(_, widths)| widths)
    }

    /// Each loaded face's fontdb ID, its font file and its index in that
    /// file.
    pub(super) fn sources(&self) -> &[(ID, Arc<[u8]>, u32)] {
        &self.sources
    }
}

/// The attributes both text users shape `family` with, so text is measured
/// exactly as it is drawn.
///
/// Kerning and ligatures are off, as QuickDraw draws one glyph per
/// character and advances each by its own width: a pair is as wide as its
/// two characters apart.
///
/// No cache-key flags are set: the renderer rasterises every glyph itself
/// and starts it on a whole physical pixel, as QuickDraw draws every glyph
/// from a whole-pixel pen ([`hinted_text`](super::hinted_text)).
pub(super) fn attrs(family: Family<'_>) -> Attrs<'_> {
    let mut features = FontFeatures::new();
    features
        .disable(FeatureTag::KERNING)
        .disable(FeatureTag::STANDARD_LIGATURES)
        .disable(FeatureTag::CONTEXTUAL_LIGATURES);
    Attrs::new().family(family).font_features(features)
}

/// A font system holding only `faces`, with the fallback as its
/// sans-serif family, and each face's family name (the fallback's,
/// Charcoal's and the serif's) and device widths.
pub(super) fn font_system(faces: &FontFaces) -> (FontSystem, Families) {
    let mut db = Database::new();
    let mut widths = Vec::new();
    let mut sources = Vec::new();
    let mut load = |bytes: &Arc<[u8]>| {
        let ids = db.load_font_source(Source::Binary(Arc::new(Arc::clone(bytes))));
        let device = DeviceWidths::read(bytes);
        widths.extend(ids.iter().map(|&id| (id, device.clone())));
        sources.extend(ids.iter().filter_map(|&id| {
            let index = db.face(id)?.index;
            Some((id, Arc::clone(bytes), index))
        }));
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
            widths,
            sources,
        },
    )
}
