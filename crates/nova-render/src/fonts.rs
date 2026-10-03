//! The font faces text is drawn in, and which face draws each [`Font`].
//!
//! The renderer never scans the system's fonts: it draws only the faces
//! it is handed in [`FontFaces`], so text looks the same on every machine.
//! [`FALLBACK_FONT`], compiled in, draws Geneva, and Charcoal when the
//! player's own Charcoal (`nova_data::fonts`) is missing or does not load.
//! A face that does not load is never drawn in: [`face_for`] picks among
//! the faces that did.

use std::sync::Arc;

use nova_view::Font;

/// The bundled substitute for Geneva (and for a missing Charcoal): Noto Sans
/// Regular, under the SIL Open Font License 1.1 (`fonts/OFL.txt`).
pub const FALLBACK_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// The font files the adapter loads: the fallback face, and Charcoal's when
/// the game data has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFaces {
    charcoal: Option<Arc<[u8]>>,
    fallback: Arc<[u8]>,
}

impl FontFaces {
    /// Just `fallback`, with no Charcoal.
    #[must_use]
    pub fn new(fallback: impl Into<Arc<[u8]>>) -> Self {
        Self {
            charcoal: None,
            fallback: fallback.into(),
        }
    }

    /// Just the bundled [`FALLBACK_FONT`].
    #[must_use]
    pub fn bundled() -> Self {
        Self::new(FALLBACK_FONT)
    }

    /// These faces with Charcoal's font file, `bytes`.
    #[must_use]
    pub fn with_charcoal(self, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            charcoal: Some(bytes.into()),
            ..self
        }
    }

    /// Charcoal's font file, if there is one.
    #[must_use]
    pub fn charcoal(&self) -> Option<&Arc<[u8]>> {
        self.charcoal.as_ref()
    }

    /// The fallback font file.
    #[must_use]
    pub fn fallback(&self) -> &Arc<[u8]> {
        &self.fallback
    }
}

/// A loaded face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// The player's Charcoal.
    Charcoal,
    /// The bundled fallback.
    Fallback,
}

/// Which of the [`FontFaces`] loaded a face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// Charcoal's font file did.
    pub charcoal: bool,
    /// The fallback font file did.
    pub fallback: bool,
}

/// The face that draws `font`, given which faces `loaded`: Charcoal draws
/// in its own face when that face loaded, and everything else in the
/// fallback. When the fallback did not load, everything draws in Charcoal;
/// when neither did, nothing draws (`None`).
#[must_use]
pub fn face_for(font: Font, loaded: Loaded) -> Option<Face> {
    match font {
        Font::Charcoal if loaded.charcoal => Some(Face::Charcoal),
        _ if loaded.fallback => Some(Face::Fallback),
        _ if loaded.charcoal => Some(Face::Charcoal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use nova_data::fonts::check_sfnt;

    use super::*;

    #[test]
    fn charcoal_draws_in_its_own_face_only_when_it_loaded() {
        let both = Loaded {
            charcoal: true,
            fallback: true,
        };
        let fallback_only = Loaded {
            charcoal: false,
            fallback: true,
        };
        assert_eq!(face_for(Font::Charcoal, both), Some(Face::Charcoal));
        assert_eq!(
            face_for(Font::Charcoal, fallback_only),
            Some(Face::Fallback)
        );
        assert_eq!(face_for(Font::Geneva, both), Some(Face::Fallback));
        assert_eq!(face_for(Font::Geneva, fallback_only), Some(Face::Fallback));
    }

    #[test]
    fn with_no_fallback_face_everything_draws_in_charcoal() {
        let charcoal_only = Loaded {
            charcoal: true,
            fallback: false,
        };
        assert_eq!(
            face_for(Font::Charcoal, charcoal_only),
            Some(Face::Charcoal)
        );
        assert_eq!(face_for(Font::Geneva, charcoal_only), Some(Face::Charcoal));
    }

    #[test]
    fn with_no_face_at_all_nothing_draws() {
        let none = Loaded {
            charcoal: false,
            fallback: false,
        };
        assert_eq!(face_for(Font::Charcoal, none), None);
        assert_eq!(face_for(Font::Geneva, none), None);
    }

    #[test]
    fn the_bundled_faces_are_the_fallback_alone() {
        let faces = FontFaces::bundled();
        assert_eq!(&faces.fallback()[..], FALLBACK_FONT);
        assert_eq!(faces.charcoal(), None);
        assert_eq!(faces, FontFaces::new(FALLBACK_FONT));
    }

    #[test]
    fn the_bundled_fallback_is_a_usable_outline_font() {
        assert_eq!(check_sfnt(FALLBACK_FONT), Ok(()));
        assert_eq!(FALLBACK_FONT.len(), 556_216);
    }

    #[test]
    fn charcoal_is_added_beside_the_fallback() {
        let faces = FontFaces::new(&b"fallback"[..]).with_charcoal(&b"charcoal"[..]);
        assert_eq!(&faces.fallback()[..], b"fallback");
        assert_eq!(faces.charcoal().map(|c| &c[..]), Some(&b"charcoal"[..]));
    }
}
