//! The font faces text is drawn in, and which face draws each [`Font`].
//!
//! The renderer never scans the system's fonts: it draws only the faces
//! it is handed in [`FontFaces`], so text looks the same on every machine.
//! There are three faces. [`FALLBACK_FONT`], compiled in, draws Geneva,
//! and Charcoal when the player's own Charcoal (`nova_data::fonts`) is
//! missing or does not load. [`SERIF_FONT`], compiled in, draws Times
//! (QuickDraw font 20), which EV Nova.app does not ship. A face that does
//! not load is never drawn in: [`face_for`] picks among the faces that
//! did.
//!
//! Every glyph advances a whole unit, as QuickDraw's do with fractional
//! widths off: [`whole_advance`] gives a glyph its face's device width
//! ([`device_width`], from the face's `hdmx`) when there is one, else its
//! linear advance rounded.

use std::sync::Arc;

use nova_data::fonts::DeviceWidths;
use nova_view::Font;

/// The bundled substitute for Geneva (and for a missing Charcoal): Noto Sans
/// Regular, under the SIL Open Font License 1.1 (`fonts/OFL.txt`).
pub const FALLBACK_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// The bundled stand-in for font 20, Times: Noto Serif Regular, under the
/// SIL Open Font License 1.1 (`fonts/NotoSerif-OFL.txt`).
pub const SERIF_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Regular.ttf");

/// The font files the adapter loads: the fallback face, the serif when
/// there is one, and Charcoal's when the game data has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFaces {
    charcoal: Option<Arc<[u8]>>,
    fallback: Arc<[u8]>,
    serif: Option<Arc<[u8]>>,
}

impl FontFaces {
    /// Just `fallback`, with no Charcoal and no serif.
    #[must_use]
    pub fn new(fallback: impl Into<Arc<[u8]>>) -> Self {
        Self {
            charcoal: None,
            fallback: fallback.into(),
            serif: None,
        }
    }

    /// The bundled faces: [`FALLBACK_FONT`] and, for Times,
    /// [`SERIF_FONT`].
    #[must_use]
    pub fn bundled() -> Self {
        Self::new(FALLBACK_FONT).with_serif(SERIF_FONT)
    }

    /// These faces with the serif's font file, `bytes`, which draws Times.
    #[must_use]
    pub fn with_serif(self, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            serif: Some(bytes.into()),
            ..self
        }
    }

    /// The serif's font file, if there is one.
    #[must_use]
    pub fn serif(&self) -> Option<&Arc<[u8]>> {
        self.serif.as_ref()
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
    /// The bundled serif.
    Serif,
}

/// Which of the [`FontFaces`] loaded a face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// Charcoal's font file did.
    pub charcoal: bool,
    /// The fallback font file did.
    pub fallback: bool,
    /// The serif's font file did.
    pub serif: bool,
}

/// The face that draws `font`, given which faces `loaded`: Charcoal draws
/// in its own face when that face loaded, Times in the serif when it
/// loaded, and everything else in the fallback. When the fallback did not
/// load, everything else draws in Charcoal; when neither did, nothing
/// draws (`None`).
#[must_use]
pub fn face_for(font: Font, loaded: Loaded) -> Option<Face> {
    match font {
        Font::Charcoal if loaded.charcoal => Some(Face::Charcoal),
        Font::Times if loaded.serif => Some(Face::Serif),
        _ if loaded.fallback => Some(Face::Fallback),
        _ if loaded.charcoal => Some(Face::Charcoal),
        _ => None,
    }
}

/// A glyph's whole advance, in logical units: its `device` width when its
/// face has one at the size, else its `linear` (unhinted) advance
/// rounded, as QuickDraw advances a glyph with fractional widths off.
#[must_use]
pub fn whole_advance(linear: f32, device: Option<u8>) -> f32 {
    device.map_or_else(|| linear.round(), f32::from)
}

/// Glyph `glyph`'s device width in `widths` at `size` points: only at a
/// whole ppem (a size of 1 to 255 with no fraction, ppem being the point
/// size at 72 dpi) that the face's `hdmx` has a record for.
#[must_use]
pub fn device_width(widths: &DeviceWidths, size: f32, glyph: u16) -> Option<u8> {
    let whole = size.fract() == 0.0 && (1.0..=255.0).contains(&size);
    // In range and whole, so the cast is exact.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ppem = whole.then_some(size as u8)?;
    widths.width(ppem, glyph)
}

#[cfg(test)]
mod tests {
    use nova_data::fonts::check_sfnt;
    use nova_data::fonts::fixture::{block_sfnt, hdmx};

    use super::*;

    #[test]
    fn charcoal_draws_in_its_own_face_only_when_it_loaded() {
        let both = Loaded {
            charcoal: true,
            fallback: true,
            serif: false,
        };
        let fallback_only = Loaded {
            charcoal: false,
            fallback: true,
            serif: false,
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
            serif: false,
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
            serif: false,
        };
        assert_eq!(face_for(Font::Charcoal, none), None);
        assert_eq!(face_for(Font::Geneva, none), None);
    }

    #[test]
    fn times_draws_in_the_serif_when_it_loaded() {
        let loaded = |charcoal, fallback, serif| Loaded {
            charcoal,
            fallback,
            serif,
        };
        let all = loaded(true, true, true);
        assert_eq!(face_for(Font::Times, all), Some(Face::Serif));
        assert_eq!(face_for(Font::Geneva, all), Some(Face::Fallback));
        assert_eq!(face_for(Font::Charcoal, all), Some(Face::Charcoal));
        assert_eq!(
            face_for(Font::Times, loaded(true, true, false)),
            Some(Face::Fallback)
        );
        assert_eq!(
            face_for(Font::Times, loaded(true, false, false)),
            Some(Face::Charcoal)
        );
        assert_eq!(face_for(Font::Times, loaded(false, false, false)), None);
        let serif_only = loaded(false, false, true);
        assert_eq!(face_for(Font::Times, serif_only), Some(Face::Serif));
        assert_eq!(face_for(Font::Geneva, serif_only), None);
    }

    #[test]
    fn the_bundled_faces_are_the_fallback_and_the_serif() {
        let faces = FontFaces::bundled();
        assert_eq!(&faces.fallback()[..], FALLBACK_FONT);
        assert_eq!(faces.serif().map(|serif| &serif[..]), Some(SERIF_FONT));
        assert_eq!(faces.charcoal(), None);
        assert_eq!(faces, FontFaces::new(FALLBACK_FONT).with_serif(SERIF_FONT));
    }

    #[test]
    fn the_bundled_serif_is_a_usable_outline_font() {
        assert_eq!(check_sfnt(SERIF_FONT), Ok(()));
        assert_eq!(SERIF_FONT.len(), 482_540);
    }

    #[test]
    fn the_serif_is_added_beside_the_fallback() {
        let plain = FontFaces::new(&b"fallback"[..]);
        assert_eq!(plain.serif(), None);
        let faces = plain.with_serif(&b"serif"[..]);
        assert_eq!(&faces.fallback()[..], b"fallback");
        assert_eq!(faces.serif().map(|s| &s[..]), Some(&b"serif"[..]));
        assert_eq!(faces.charcoal(), None);
    }

    #[test]
    fn the_bundled_fallback_is_a_usable_outline_font() {
        assert_eq!(check_sfnt(FALLBACK_FONT), Ok(()));
        assert_eq!(FALLBACK_FONT.len(), 556_216);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn the_device_width_wins_over_the_linear_advance() {
        assert_eq!(whole_advance(5.46, Some(7)), 7.0);
        assert_eq!(whole_advance(6.04, Some(5)), 5.0);
        assert_eq!(whole_advance(6.04, Some(0)), 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn with_no_device_width_the_linear_advance_rounds() {
        assert_eq!(whole_advance(5.46, None), 5.0);
        assert_eq!(whole_advance(5.5, None), 6.0);
        assert_eq!(whole_advance(6.04, None), 6.0);
        assert_eq!(whole_advance(0.0, None), 0.0);
        assert_eq!(whole_advance(0.49, None), 0.0);
    }

    /// The block font's device widths: glyph 1 is 13 wide at 10 ppem and
    /// 1 wide at 255.
    fn widths() -> DeviceWidths {
        let bytes = block_sfnt("Charcoal")
            .table(b"hdmx", hdmx(2, &[(10, &[0, 13]), (255, &[0, 1])]))
            .build();
        DeviceWidths::read(&bytes)
    }

    #[test]
    fn a_device_width_is_found_at_a_whole_ppem_with_a_record() {
        let widths = widths();
        assert_eq!(device_width(&widths, 10.0, 1), Some(13));
        assert_eq!(device_width(&widths, 255.0, 1), Some(1));
        assert_eq!(device_width(&widths, 10.0, 0), Some(0));
        assert_eq!(device_width(&widths, 10.0, 2), None, "no such glyph");
        assert_eq!(device_width(&widths, 12.0, 1), None, "no record");
    }

    #[test]
    fn a_size_that_is_not_a_whole_ppem_has_no_device_width() {
        let widths = widths();
        for size in [
            10.5,
            9.999,
            10.001,
            0.0,
            -10.0,
            255.5,
            256.0,
            266.0,
            f32::NAN,
            f32::INFINITY,
        ] {
            assert_eq!(device_width(&widths, size, 1), None, "{size}");
        }
    }

    #[test]
    fn charcoal_is_added_beside_the_fallback() {
        let faces = FontFaces::new(&b"fallback"[..]).with_charcoal(&b"charcoal"[..]);
        assert_eq!(&faces.fallback()[..], b"fallback");
        assert_eq!(faces.charcoal().map(|c| &c[..]), Some(&b"charcoal"[..]));
    }
}
